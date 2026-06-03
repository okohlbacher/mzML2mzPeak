#!/usr/bin/env python3
"""Generate a tiny, valid PROCESSED imzML/.ibd fixture for CI streaming tests.

Synthesizes a 3x3-pixel processed dataset (9 spectra) where each pixel carries its OWN
m/z + intensity arrays of DIFFERING lengths (the defining property of processed mode), with:
  - m/z declared MS:1000523 (64-bit float)  -> exercises NumArray::F64
  - intensity declared MS:1000521 (32-bit float) -> exercises NumArray::F32
The .ibd is laid out as: [16-byte RFC-4122 UUID][per-spectrum mz bytes][intensity bytes]...
with each binaryDataArray's IMS:1000102 external offset / IMS:1000103 array length / IMS:1000104
encoded byte length matching the actual layout. The whole-file SHA-1 (IMS:1000091) and the
first-16-byte UUID (IMS:1000080) are computed from the emitted bytes so the pair PASSES the
Plan 02-02 integrity preflight.

Run from the repo root:  python3 tests/fixtures/imaging/gen_processed_fixture.py
Emits Example_Processed.imzML + Example_Processed.ibd next to this script.
"""
import hashlib
import os
import struct
import uuid as uuidlib

HERE = os.path.dirname(os.path.abspath(__file__))
IMZML = os.path.join(HERE, "Example_Processed.imzML")
IBD = os.path.join(HERE, "Example_Processed.ibd")

# Fixed UUID so the test can assert provenance().uuid deterministically.
UUID_STR = "0a1b2c3d-4e5f-6071-8293-a4b5c6d7e8f9"
UUID_BYTES = uuidlib.UUID(UUID_STR).bytes  # RFC-4122 / big-endian, 16 bytes

# 3x3 grid; vary the per-pixel array length so processed-mode variation is provable.
# lengths chosen distinct per pixel (DIFFER across pixels).
GRID = [(x, y) for y in (1, 2, 3) for x in (1, 2, 3)]
LENGTHS = [3, 4, 5, 6, 7, 8, 9, 10, 11]  # one per pixel, all different


def build():
    ibd = bytearray()
    ibd += UUID_BYTES  # bytes 0..16

    spectra = []  # (x, y, mz_off, mz_len, mz_enc, int_off, int_len, int_enc)
    for (x, y), n in zip(GRID, LENGTHS):
        # m/z: 64-bit float, ascending values
        mz_vals = [100.0 + x + y * 0.1 + i * 0.5 for i in range(n)]
        mz_off = len(ibd)
        mz_bytes = struct.pack("<%dd" % n, *mz_vals)  # little-endian f64
        ibd += mz_bytes
        # intensity: 32-bit float
        int_vals = [float((i + 1) * 10 + x) for i in range(n)]
        int_off = len(ibd)
        int_bytes = struct.pack("<%df" % n, *int_vals)  # little-endian f32
        ibd += int_bytes
        spectra.append(
            (x, y, mz_off, n, len(mz_bytes), int_off, n, len(int_bytes))
        )

    ibd = bytes(ibd)
    sha1 = hashlib.sha1(ibd).hexdigest()
    return ibd, sha1, spectra


def imzml(sha1, spectra):
    spec_xml = []
    for idx, (x, y, mz_off, mz_len, mz_enc, int_off, int_len, int_enc) in enumerate(spectra):
        spec_xml.append(f"""      <spectrum id="Scan={idx + 1}" defaultArrayLength="0" index="{idx}">
        <referenceableParamGroupRef ref="spectrum1"/>
        <cvParam cvRef="MS" accession="MS:1000285" name="total ion current" value="1.0"/>
        <scanList count="1">
          <cvParam cvRef="MS" accession="MS:1000795" name="no combination"/>
          <scan>
            <cvParam cvRef="IMS" accession="IMS:1000050" name="position x" value="{x}"/>
            <cvParam cvRef="IMS" accession="IMS:1000051" name="position y" value="{y}"/>
          </scan>
        </scanList>
        <binaryDataArrayList count="2">
          <binaryDataArray encodedLength="0">
            <referenceableParamGroupRef ref="mzArray"/>
            <cvParam cvRef="IMS" accession="IMS:1000103" name="external array length" value="{mz_len}"/>
            <cvParam cvRef="IMS" accession="IMS:1000102" name="external offset" value="{mz_off}"/>
            <cvParam cvRef="IMS" accession="IMS:1000104" name="external encoded length" value="{mz_enc}"/>
            <binary />
          </binaryDataArray>
          <binaryDataArray encodedLength="0">
            <referenceableParamGroupRef ref="intensityArray"/>
            <cvParam cvRef="IMS" accession="IMS:1000103" name="external array length" value="{int_len}"/>
            <cvParam cvRef="IMS" accession="IMS:1000102" name="external offset" value="{int_off}"/>
            <cvParam cvRef="IMS" accession="IMS:1000104" name="external encoded length" value="{int_enc}"/>
            <binary />
          </binaryDataArray>
        </binaryDataArrayList>
      </spectrum>""")
    spectra_block = "\n".join(spec_xml)
    return f"""<?xml version="1.0" encoding="ISO-8859-1"?>
<mzML xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://psi.hupo.org/ms/mzml http://psidev.info/files/ms/mzML/xsd/mzML1.1.0.xsd" xmlns="http://psi.hupo.org/ms/mzml" version="1.1">
  <cvList count="3">
    <cv URI="https://raw.githubusercontent.com/hupo-psi/psi-ms-cv/master/psi-ms.obo" fullName="Proteomics Standards Initiative Mass Spectrometry Ontology" id="MS" version="4.1.0"/>
    <cv URI="https://raw.githubusercontent.com/imzML/imzML/master/imagingMS.obo" fullName="Mass Spectrometry Imaging Ontology" id="IMS" version="1.1.0"/>
    <cv URI="http://ontologies.berkeleybop.org/uo.obo" fullName="Units of Measurement Ontology" id="UO" version="releases/2017-09-25"/>
  </cvList>
  <fileDescription>
    <fileContent>
      <cvParam cvRef="MS" accession="MS:1000579" name="MS1 spectrum"/>
      <cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/>
      <cvParam cvRef="IMS" accession="IMS:1000080" name="universally unique identifier" value="{{{UUID_STR}}}"/>
      <cvParam cvRef="IMS" accession="IMS:1000091" name="ibd SHA-1" value="{sha1}"/>
      <cvParam cvRef="IMS" accession="IMS:1000031" name="processed"/>
    </fileContent>
  </fileDescription>
  <referenceableParamGroupList count="3">
    <referenceableParamGroup id="mzArray">
      <cvParam cvRef="MS" accession="MS:1000576" name="no compression"/>
      <cvParam cvRef="MS" accession="MS:1000514" name="m/z array" unitCvRef="MS" unitAccession="MS:1000040" unitName="m/z"/>
      <cvParam cvRef="IMS" accession="IMS:1000101" name="external data" value="true"/>
      <cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/>
    </referenceableParamGroup>
    <referenceableParamGroup id="intensityArray">
      <cvParam cvRef="MS" accession="MS:1000576" name="no compression"/>
      <cvParam cvRef="MS" accession="MS:1000515" name="intensity array" unitCvRef="MS" unitAccession="MS:1000131" unitName="number of detector counts"/>
      <cvParam cvRef="IMS" accession="IMS:1000101" name="external data" value="true"/>
      <cvParam cvRef="MS" accession="MS:1000521" name="32-bit float"/>
    </referenceableParamGroup>
    <referenceableParamGroup id="spectrum1">
      <cvParam cvRef="MS" accession="MS:1000579" name="MS1 spectrum"/>
      <cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/>
      <cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/>
    </referenceableParamGroup>
  </referenceableParamGroupList>
  <run id="ExperimentProcessed">
    <spectrumList count="{len(spectra)}" defaultDataProcessingRef="dp1">
{spectra_block}
    </spectrumList>
  </run>
</mzML>
"""


def main():
    ibd, sha1, spectra = build()
    with open(IBD, "wb") as f:
        f.write(ibd)
    xml = imzml(sha1, spectra)
    # imzML header is ISO-8859-1; our content is pure ASCII so latin-1 == ascii here.
    with open(IMZML, "wb") as f:
        f.write(xml.encode("latin-1"))
    print(f"wrote {IBD} ({len(ibd)} bytes), sha1={sha1}")
    print(f"wrote {IMZML} ({len(xml)} bytes), {len(spectra)} spectra")


if __name__ == "__main__":
    main()
