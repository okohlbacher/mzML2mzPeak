# Creating deep links to the mzPeak viewers

After mzML2mzPeak converts a file to `.mzpeak` and it is uploaded to a public,
range-capable object store, you can emit a **deep link** that opens the file
directly in a browser-based viewer — no upload, no clicks. This is meant to be
produced automatically by the conversion/publish pipeline (e.g. printed in the
job output, written to a manifest, or embedded in a report).

There are two viewers; both take the same query parameter:

| Viewer | Base URL | Best for |
|---|---|---|
| **mzPeak Explorer** | `https://okohlbacher.github.io/mzPeakExplorer/` | LC–MS / general files: summary, metadata tree, spectra, chromatograms, ZIP/parquet structure |
| **mzPeakIV** | `https://okohlbacher.github.io/mzPeakIV/` | imaging (MSI) files: ion images, optical overlays |

> Pick the viewer by content: imaging `.mzpeak` → mzPeakIV; everything else →
> mzPeak Explorer. (Both will *open* any `.mzpeak`, but each is specialised.)

---

## URL format

```
<viewer-base>?file=<percent-encoded absolute URL of the .mzpeak>
```

- Parameter name: **`file`** (alias **`url`** is also accepted).
- The value is the **full absolute URL** to the `.mzpeak` object, **percent-encoded**.
- The viewer streams the file via HTTP range requests — it reads only the ZIP
  footer + the parts the user views, so even multi-hundred-MB files open quickly.

### Example

File:
```
https://object.storage.eu01.onstackit.cloud/v09/mzML-examples/sciex-tripletof-6600/12_80.mzpeak
```

Deep link (mzPeak Explorer):
```
https://okohlbacher.github.io/mzPeakExplorer/?file=https%3A%2F%2Fobject.storage.eu01.onstackit.cloud%2Fv09%2FmzML-examples%2Fsciex-tripletof-6600%2F12_80.mzpeak
```

Same file in mzPeakIV — just swap the base:
```
https://okohlbacher.github.io/mzPeakIV/?file=https%3A%2F%2Fobject.storage.eu01.onstackit.cloud%2Fv09%2FmzML-examples%2Fsciex-tripletof-6600%2F12_80.mzpeak
```

### `s3://` shorthand (mzPeakIV only)

mzPeakIV additionally accepts an `s3://bucket/key` value, which it rewrites to the
configured BL-S3 HTTPS endpoint before fetching:
```
https://okohlbacher.github.io/mzPeakIV/?file=s3%3A%2F%2Fv09%2FmzML-examples%2F…%2F12_80.mzpeak
```
mzPeak Explorer expects an explicit `http(s)` URL — prefer the full HTTPS form for
links that should work in **both** viewers.

---

## Hard requirements on the hosted object

A deep link only works if the object store serves the file with:

1. **Range requests** — `Accept-Ranges: bytes`, `206 Partial Content`.
2. **CORS** allowing the viewer origin `https://okohlbacher.github.io`
   (or `*`), with the `Range` request header allowed and `Content-Range` /
   `Accept-Ranges` exposed.
3. **Public read** (anonymous `GET`), and **no `Content-Encoding: gzip`**
   (an `.mzpeak` is already a ZIP; a gzip transfer-encoding breaks byte ranges).
   Serve as `application/zip` or `binary/octet-stream`.

These are once-per-bucket settings. Example S3-compatible config (StackIT, R2, S3):

CORS:
```json
[{
  "AllowedOrigins": ["https://okohlbacher.github.io"],
  "AllowedMethods": ["GET", "HEAD"],
  "AllowedHeaders": ["Range", "If-Range", "*"],
  "ExposeHeaders": ["Content-Range", "Accept-Ranges", "Content-Length", "ETag"],
  "MaxAgeSeconds": 3600
}]
```
Public-read bucket policy (scope `Resource` to the published prefix):
```json
{ "Version": "2012-10-17", "Statement": [{
  "Sid": "PublicReadMzpeak", "Effect": "Allow", "Principal": "*",
  "Action": "s3:GetObject",
  "Resource": "arn:aws:s3:::<bucket>/<prefix>/*"
}]}
```

### Verify a link before publishing it
```sh
URL="<object url>"
curl -sI -H "Range: bytes=0-1023" "$URL" | grep -iE '^HTTP|content-range|accept-ranges'
curl -sI -X OPTIONS -H "Origin: https://okohlbacher.github.io" \
     -H "Access-Control-Request-Method: GET" \
     -H "Access-Control-Request-Headers: range" "$URL" | grep -i access-control
```
Expect `206` + `Content-Range`/`Accept-Ranges` on the first, and an
`Access-Control-Allow-Origin` + `Access-Control-Allow-Headers: range` on the second.

---

## Building the link in code (Rust)

Percent-encode the **whole** object URL as the `file` value. Don't hand-roll the
encoding — use a crate so `:`, `/`, `?`, `&`, `%` are all escaped.

Using the `url` crate (already transitively common) — lets `query_pairs_mut` do
the encoding:
```rust
use url::Url;

/// Build a viewer deep link for an already-uploaded .mzpeak object URL.
pub fn deep_link(viewer_base: &str, object_url: &str) -> String {
    let mut u = Url::parse(viewer_base).expect("valid viewer base");
    u.query_pairs_mut().clear().append_pair("file", object_url);
    u.into()
}

// mzPeak Explorer
const EXPLORER: &str = "https://okohlbacher.github.io/mzPeakExplorer/";
// mzPeakIV (imaging)
const MZPEAKIV: &str = "https://okohlbacher.github.io/mzPeakIV/";

// deep_link(EXPLORER, "https://object.storage.eu01.onstackit.cloud/v09/…/12_80.mzpeak")
```

Or with `percent-encoding` (no `url` dependency):
```rust
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

pub fn deep_link(viewer_base: &str, object_url: &str) -> String {
    let enc = utf8_percent_encode(object_url, NON_ALPHANUMERIC).to_string();
    format!("{viewer_base}?file={enc}")
}
```

> `NON_ALPHANUMERIC` over-encodes a little (e.g. `-._~`), which is harmless — the
> viewer decodes with `URLSearchParams`, which accepts it.

### Picking the viewer programmatically
If the converter knows the file is imaging (the mzPeak index block declares
imaging / IMS coordinates), point the link at `mzPeakIV`; otherwise `mzPeakExplorer`.

---

## Summary for the pipeline

1. Convert `*.mzML` → `*.mzpeak` (mzML2mzPeak).
2. Upload to the public, CORS+range bucket prefix.
3. `deep_link(<viewer base>, <object https url>)` → print/emit the link.
4. (Once per bucket) ensure the CORS + public-read + no-gzip settings above.
