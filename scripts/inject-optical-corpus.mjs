// Inject optical image sidecar(s) into each image-bearing imzML example via --image,
// --verify the spectral L1 round-trip, and report the embedded images[] per archive.
import { execFileSync, execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const ROOT = "data/imzml-examples";
const BIN = "target/release/mzml2mzpeak";
const OUT = "out/optical-demo";
fs.mkdirSync(OUT, { recursive: true });
const IMG = /\.(tif|tiff|png|jpg|jpeg|svs)$/i;

function walk(d, acc = []) {
  for (const e of fs.readdirSync(d, { withFileTypes: true })) {
    const p = path.join(d, e.name);
    e.isDirectory() ? walk(p, acc) : acc.push(p);
  }
  return acc;
}
const all = walk(ROOT);
const imzs = all.filter((f) => /\.imzML$/i.test(f)).sort();

const jobs = [];
for (const imz of imzs) {
  let sec = path.dirname(imz);
  if (path.basename(sec).toLowerCase() === "imzml") sec = path.dirname(sec);
  const imgs = all.filter((f) => IMG.test(f) && f.startsWith(sec + path.sep)).sort();
  if (imgs.length) jobs.push({ imz, imgs, sec });
}

const rows = [];
for (const { imz, imgs } of jobs) {
  const ds = imz.slice(ROOT.length + 1).split(path.sep)[0];
  const tag = imz.slice(ROOT.length + 1).replace(/[ ,/]/g, "_").replace(/\.imzML$/i, "").slice(0, 80);
  const mz = path.join(OUT, tag + ".mzpeak");
  const args = [imz, mz, "--verify"];
  for (const g of imgs) args.push("--image", g);
  process.stderr.write(`>>> ${ds}  (${imgs.length} image${imgs.length > 1 ? "s" : ""}: ${imgs.map((g) => path.extname(g)).join(",")})\n`);
  let exit = 0, note = "", members = [], imagesMeta = [];
  const t0 = Date.now();
  // Stream stderr to a file (the converter can emit one warning PER spectrum →
  // tens of thousands of lines, which overflows execFileSync's in-memory maxBuffer
  // and falsely kills the process). Writing to a file avoids that entirely.
  const errfile = path.join(OUT, tag + ".stderr.log");
  const efd = fs.openSync(errfile, "w");
  try {
    execFileSync(BIN, args, { stdio: ["ignore", "ignore", efd] });
  } catch (e) {
    exit = e.status ?? -1;
  } finally {
    fs.closeSync(efd);
  }
  if (exit !== 0) {
    const err = fs.readFileSync(errfile, "utf8").split("\n");
    note = err.filter((l) => /error|panic|fail|mismatch|unsupported|BigTIFF|integrity/i.test(l) && !/reading_shared|dateTime/.test(l)).slice(-1)[0]?.slice(0, 110) || err.slice(-2)[0]?.slice(0, 110) || "";
  }
  const secs = ((Date.now() - t0) / 1000).toFixed(0);
  if (exit === 0 && fs.existsSync(mz)) {
    members = execSync(`unzip -l "${mz}" 2>/dev/null | grep -E "images/" || true`).toString().trim().split("\n").filter(Boolean)
      .map((l) => l.trim().split(/\s+/).slice(3).join(" "));
    try {
      const idx = JSON.parse(execSync(`unzip -p "${mz}" mzpeak_index.json 2>/dev/null`).toString());
      imagesMeta = idx.metadata?.imaging?.images || [];
    } catch {}
  }
  rows.push({ ds, imgs: imgs.map((g) => path.basename(g)), exit, secs, members, imagesMeta, mz, note });
}

// Report
console.log("\n================= OPTICAL INJECTION RESULTS =================\n");
for (const r of rows) {
  const ok = r.exit === 0 && r.members.length === r.imgs.length;
  console.log(`${ok ? "✅" : "❌"} ${r.ds}  (verify exit ${r.exit}, ${r.secs}s)`);
  console.log(`   source images: ${r.imgs.join("  |  ")}`);
  if (r.members.length) {
    for (const m of r.imagesMeta) {
      console.log(`     → ${m.archive_path}  ${m.media_type}  ${m.width}×${m.height}  role=${m.role}  ${(m.size_bytes / 1048576).toFixed(1)}MB  sha=${(m.sha256 || "").slice(0, 12)}…`);
    }
    console.log(`     members in ZIP: ${r.members.join(", ")}`);
  } else if (r.exit !== 0) {
    console.log(`     ✗ ${r.note || "conversion failed"}`);
  }
  console.log("");
}
const okN = rows.filter((r) => r.exit === 0 && r.members.length === r.imgs.length).length;
console.log(`Injected ${okN}/${rows.length} datasets;  multi-image: ${rows.filter((r) => r.imgs.length > 1).map((r) => r.ds).join(", ") || "none succeeded"}`);
