#!/usr/bin/env python3
"""Generate a tiny, valid PROCESSED imzML/.ibd fixture with a DECLARED grid for CI round-trip tests.

Synthesizes a 3x3-pixel processed dataset (9 spectra) where each pixel carries its OWN
m/z + intensity arrays of DIFFERING lengths (the defining property of processed mode), with:
  - m/z declared MS:1000523 (64-bit float)  -> exercises NumArray::F64
  - intensity declared MS:1000521 (32-bit float) -> exercises NumArray::F32

The STRUCTURAL ADDITION vs gen_processed_fixture.py: a <scanSettingsList>/<scanSettings> block
is emitted BEFORE the <run>, declaring:
  - IMS:1000042 / IMS:1000043 — max count of pixels x/y = 3 (the declared grid)
  - IMS:1000046 / IMS:1000047 — pixel size x/y = 100.0 µm (UO:0000017)
  - IMS:1000044 / IMS:1000045 — max dimension x/y = 300 µm (UO:0000017)

The declared grid is CONSISTENT with the emitted pixel coordinates (1,1)..(3,3): the observed
max is exactly 3×3, matching the declared grid. No inconsistency warning; this is the HAPPY
declared-geometry path (GEOF-01 end-to-end proof fixture).

The .ibd layout, IMS:1000102/103/104 external-array encoding, and UUID/SHA-1 integrity wiring
are IDENTICAL to gen_processed_fixture.py so the pair PASSES the Plan 02-02 integrity preflight.

Run from the repo root:  python3 tests/fixtures/imaging/gen_declared_geometry_fixture.py
Emits Synthetic_DeclaredGrid.imzML + Synthetic_DeclaredGrid.ibd next to this script.
"""
import hashlib
import os
import struct
import uuid as uuidlib

HERE = os.path.dirname(os.path.abspath(__file__))
IMZML = os.path.join(HERE, "Synthetic_DeclaredGrid.imzML")
IBD = os.path.join(HERE, "Synthetic_DeclaredGrid.ibd")

# Fixed UUID so the test can assert provenance().uuid deterministically.
UUID_STR = "1a2b3c4d-5e6f-7081-9203-b4c5d6e7f8a9"
UUID_BYTES = uuidlib.UUID(UUID_STR).bytes  # RFC-4122 / big-endian, 16 bytes

# 3x3 grid; vary the per-pixel array length so processed-mode variation is provable.
# Lengths are distinct per pixel (DIFFER across pixels — same pattern as Example_Processed).
GRID = [(x, y) for y in (1, 2, 3) for x in (1, 2, 3)]
LENGTHS = [3, 4, 5, 6, 7, 8, 9, 10, 11]  # one per pixel, all different

# Declared grid geometry (CONSISTENT with GRID above: max x=3, max y=3).
DECLARED_GRID_X = 3
DECLARED_GRID_Y = 3
PIXEL_SIZE_UM = 100.0          # µm, x and y equal
MAX_DIM_UM = 300               # µm, x and y equal (= pixel_size * grid_count)


def build():
    ibd = bytearray()
    ibd += UUID_BYTES  # bytes 0..16

    spectra = []  # (x, y, mz_off, mz_len, mz_enc, int_off, int_len, int_enc)
    for (x, y), n in zip(GRID, LENGTHS):
        # m/z: 64-bit float, ascending values
        mz_vals = [200.0 + x + y * 0.1 + i * 0.5 for i in range(n)]
        mz_off = len(ibd)
        mz_bytes = struct.pack("<%dd" % n, *mz_vals)  # little-endian f64
        ibd += mz_bytes
        # intensity: 32-bit float
        int_vals = [float((i + 1) * 20 + x) for i in range(n)]
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
  <run id="ExperimentDeclaredGrid">
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
