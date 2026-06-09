#!/usr/bin/env python3
"""Generate a tiny PROCESSED imzML/.ibd fixture whose DECLARED grid is INCONSISTENT with the
observed pixel coordinates — the Phase-25 inconsistency-guard (GEOF-01) input.

Identical layout to gen_declared_geometry_fixture.py EXCEPT the declared grid is 2x2 while the
emitted pixel coordinates run (1,1)..(3,3) — observed max 3x3 EXCEEDS the declared 2x2 on both
axes. The forward converter's `fold_into` guard therefore OVERWRITES `pixel_count` with the
observed maxima and records `pixel_count_source == "observed_max"` (the declared grid is NOT
trusted; it must never silently overwrite the truth the pixels themselves prove).

This is the FIX-3 regression fixture: a reverse conversion of the produced archive must NOT
re-declare the observed extents as DECLARED IMS:1000042/43 max-count geometry.

Run from the repo root:  python3 tests/fixtures/imaging/gen_inconsistent_geometry_fixture.py
Emits Synthetic_InconsistentGrid.imzML + Synthetic_InconsistentGrid.ibd next to this script.
"""
import hashlib
import os
import struct
import uuid as uuidlib

HERE = os.path.dirname(os.path.abspath(__file__))
IMZML = os.path.join(HERE, "Synthetic_InconsistentGrid.imzML")
IBD = os.path.join(HERE, "Synthetic_InconsistentGrid.ibd")

# Fixed UUID for deterministic provenance assertions.
UUID_STR = "2b3c4d5e-6f70-8192-a3b4-c5d6e7f8a9b0"
UUID_BYTES = uuidlib.UUID(UUID_STR).bytes

# 3x3 emitted coordinates — observed max is 3x3.
GRID = [(x, y) for y in (1, 2, 3) for x in (1, 2, 3)]
LENGTHS = [3, 4, 5, 6, 7, 8, 9, 10, 11]

# Declared grid INTENTIONALLY TOO SMALL (2x2) — inconsistent with the observed 3x3 coords.
DECLARED_GRID_X = 2
DECLARED_GRID_Y = 2
PIXEL_SIZE_UM = 100.0
MAX_DIM_UM = 200  # = pixel_size * declared_count (2)


def build():
    ibd = bytearray()
    ibd += UUID_BYTES

    spectra = []
    for (x, y), n in zip(GRID, LENGTHS):
        mz_vals = [200.0 + x + y * 0.1 + i * 0.5 for i in range(n)]
        mz_off = len(ibd)
        mz_bytes = struct.pack("<%dd" % n, *mz_vals)
        ibd += mz_bytes
        int_vals = [float((i + 1) * 20 + x) for i in range(n)]
        int_off = len(ibd)
        int_bytes = struct.pack("<%df" % n, *int_vals)
        ibd += int_bytes
        spectra.append((x, y, mz_off, n, len(mz_bytes), int_off, n, len(int_bytes)))

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
  <scanSettingsList count="1">
    <scanSettings id="scansettings1">
      <cvParam cvRef="IMS" accession="IMS:1000042" name="max count of pixels x" value="{DECLARED_GRID_X}"/>
      <cvParam cvRef="IMS" accession="IMS:1000043" name="max count of pixels y" value="{DECLARED_GRID_Y}"/>
      <cvParam cvRef="IMS" accession="IMS:1000044" name="max dimension x" value="{MAX_DIM_UM}" unitCvRef="UO" unitAccession="UO:0000017" unitName="micrometer"/>
      <cvParam cvRef="IMS" accession="IMS:1000045" name="max dimension y" value="{MAX_DIM_UM}" unitCvRef="UO" unitAccession="UO:0000017" unitName="micrometer"/>
      <cvParam cvRef="IMS" accession="IMS:1000046" name="pixel size x" value="{PIXEL_SIZE_UM}" unitCvRef="UO" unitAccession="UO:0000017" unitName="micrometer"/>
      <cvParam cvRef="IMS" accession="IMS:1000047" name="pixel size y" value="{PIXEL_SIZE_UM}" unitCvRef="UO" unitAccession="UO:0000017" unitName="micrometer"/>
    </scanSettings>
  </scanSettingsList>
  <run id="ExperimentInconsistentGrid">
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
    with open(IMZML, "wb") as f:
        f.write(xml.encode("latin-1"))
    print(f"wrote {IBD} ({len(ibd)} bytes), sha1={sha1}")
    print(f"wrote {IMZML} ({len(xml)} bytes), {len(spectra)} spectra")


if __name__ == "__main__":
    main()
