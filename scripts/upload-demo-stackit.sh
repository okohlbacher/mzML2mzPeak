#!/usr/bin/env bash
# Upload an mzPeak demo file to STACKIT Object Storage (S3-compatible) and expose it
# as a permanent public-read download.
#
# STACKIT hierarchy:  Organization -> Folder -> Project -> Object Storage -> Bucket -> Object
#   - Buckets + credentials are scoped per *Project* (a "folder-ID" is one level above and is
#     NOT usable for S3 auth).
#   - The S3 API authenticates with an ACCESS KEY + SECRET KEY minted in a "credentials group"
#     (STACKIT Portal -> Object Storage -> Credentials & Groups -> Create credentials).
#
# This script does NOT hardcode secrets — it reads them from the environment.
#
# Required env:
#   AWS_ACCESS_KEY_ID       STACKIT Object Storage access key
#   AWS_SECRET_ACCESS_KEY   STACKIT Object Storage secret key
#
# Optional env (defaults shown):
#   BUCKET=mzpeak-demo               target bucket (must already exist unless CREATE_BUCKET=1)
#   REGION=eu01                      eu01 or eu02
#   ENDPOINT=https://object.storage.${REGION}.onstackit.cloud
#   KEY_PREFIX=demo                  object key prefix; the file is uploaded to <KEY_PREFIX>/<basename>
#   FILE=data/mzpeak/PXD001283-HR2MSI-urinary-bladder_HR2MSImouseurinarybladderS096.mzpeak
#   CREATE_BUCKET=0                  set to 1 to attempt bucket creation via the S3 API
#   PUBLIC=1                         set to 0 to skip the public-read bucket policy (upload only)
#
# Usage:
#   AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=... bash scripts/upload-demo-stackit.sh
#   # custom file/bucket:
#   BUCKET=my-bucket FILE=out/HR2MSI.mzpeak AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=... \
#     bash scripts/upload-demo-stackit.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# --- config -----------------------------------------------------------------
BUCKET="${BUCKET:-mzpeak-demo}"
REGION="${REGION:-eu01}"
ENDPOINT="${ENDPOINT:-https://object.storage.${REGION}.onstackit.cloud}"
KEY_PREFIX="${KEY_PREFIX:-demo}"
FILE="${FILE:-$ROOT/data/mzpeak/PXD001283-HR2MSI-urinary-bladder_HR2MSImouseurinarybladderS096.mzpeak}"
CREATE_BUCKET="${CREATE_BUCKET:-0}"
PUBLIC="${PUBLIC:-1}"

export AWS_REGION="$REGION" AWS_DEFAULT_REGION="$REGION"
AWS=(aws --endpoint-url "$ENDPOINT")

# --- preflight --------------------------------------------------------------
command -v aws >/dev/null || { echo "ERROR: aws CLI not found. Install with: brew install awscli" >&2; exit 1; }
: "${AWS_ACCESS_KEY_ID:?set AWS_ACCESS_KEY_ID (STACKIT Object Storage access key)}"
: "${AWS_SECRET_ACCESS_KEY:?set AWS_SECRET_ACCESS_KEY (STACKIT Object Storage secret key)}"
[ -f "$FILE" ] || { echo "ERROR: file not found: $FILE" >&2; exit 1; }

base="$(basename "$FILE")"
key="${KEY_PREFIX:+$KEY_PREFIX/}$base"
size="$(stat -f%z "$FILE" 2>/dev/null || stat -c%s "$FILE")"

echo "STACKIT Object Storage upload"
echo "  endpoint : $ENDPOINT  (region $REGION)"
echo "  bucket   : $BUCKET"
echo "  file     : $FILE  ($size bytes)"
echo "  key      : s3://$BUCKET/$key"
echo

# --- bucket -----------------------------------------------------------------
if "${AWS[@]}" s3api head-bucket --bucket "$BUCKET" 2>/dev/null; then
  echo "bucket exists: $BUCKET"
elif [ "$CREATE_BUCKET" = "1" ]; then
  echo "creating bucket via S3 API: $BUCKET"
  "${AWS[@]}" s3api create-bucket --bucket "$BUCKET" \
    || { echo "ERROR: S3 CreateBucket failed. Create it via the STACKIT Portal or:" >&2
         echo "       stackit object-storage bucket create $BUCKET -p <PROJECT_ID>" >&2; exit 1; }
else
  echo "ERROR: bucket '$BUCKET' not accessible. Create it first (Portal or):" >&2
  echo "       stackit object-storage bucket create $BUCKET -p <PROJECT_ID>" >&2
  echo "   or re-run this script with CREATE_BUCKET=1" >&2
  exit 1
fi

# --- upload (multipart is automatic for large files) ------------------------
echo "uploading..."
"${AWS[@]}" s3 cp "$FILE" "s3://$BUCKET/$key"
echo "uploaded."

# --- public-read bucket policy ----------------------------------------------
# NOTE: STACKIT uses the StorageGRID resource scheme  urn:sgws:s3:::  (NOT arn:aws:s3:::).
# put-bucket-policy REPLACES any existing policy on the bucket.
if [ "$PUBLIC" = "1" ]; then
  pol="$(mktemp)"; trap 'rm -f "$pol"' EXIT
  cat > "$pol" <<JSON
{
  "Statement": [
    {
      "Sid": "public-read-demo",
      "Effect": "Allow",
      "Principal": "*",
      "Action": "s3:GetObject",
      "Resource": "urn:sgws:s3:::$BUCKET/${KEY_PREFIX:+$KEY_PREFIX/}*"
    }
  ]
}
JSON
  echo "applying public-read policy (scope: ${KEY_PREFIX:-<root>}/*) ..."
  "${AWS[@]}" s3api put-bucket-policy --bucket "$BUCKET" --policy "file://$pol" \
    && echo "policy applied." \
    || { echo "WARN: aws put-bucket-policy failed. STACKIT documents s3cmd setpolicy as the" >&2
         echo "      supported route: s3cmd setpolicy $pol s3://$BUCKET" >&2; }
fi

echo
echo "Public download URL:"
echo "  $ENDPOINT/$BUCKET/$key"
