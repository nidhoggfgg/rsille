#!/usr/bin/env bash
# Attach the crates.io archive and any packaged binaries to the tag's GitHub Release.
set -euo pipefail

package=$1
version=$2
notes=$3
checksum_file=$4
latest=$5
shift 5

(
  cd target/package
  archive="$package-$version.crate"
  # Workspace packaging can write a different Cargo.lock from the uploaded archive.
  # Fetch the published bytes on the first run as well as on retries.
  curl_flags=(-fsSL --retry 5 --retry-all-errors --retry-max-time 90
    -A "rsille release ($GITHUB_SERVER_URL/$GITHUB_REPOSITORY)")
  checksum=$(curl "${curl_flags[@]}" "https://crates.io/api/v1/crates/$package/$version" |
    jq -er '.version.checksum | select(test("^[0-9a-f]{64}$"))')
  curl "${curl_flags[@]}" "https://static.crates.io/crates/$package/$archive" -o "$archive"
  echo "$checksum  $archive" > "$checksum_file"
  for file in "$@"; do
    sha256sum "${file##*/}" >> "$checksum_file"
  done
  sha256sum -c "$checksum_file"
  tar -xzOf "$archive" "$package-$version/.cargo_vcs_info.json" |
    jq -e --arg commit "$(git rev-parse HEAD)" '.git.sha1 == $commit and .git.dirty != true' >/dev/null ||
    { echo "::error::Published $archive does not match the release commit"; exit 1; }
)

# Listing releases also finds drafts left by interrupted uploads.
release=$(gh api --paginate "repos/$GH_REPO/releases?per_page=100" \
  --jq '.[]' | jq -sc --arg tag "$RELEASE_TAG" 'map(select(.tag_name == $tag)) |
    if length > 1 then error("multiple releases for the same tag") else .[0] end')
if [[ "$release" == null ]]; then
  gh release create "$RELEASE_TAG" --draft --verify-tag \
    --title "$package $version" --notes-file "$notes"
fi

draft=$(jq -r 'if . == null then true else .draft end' <<< "$release")
for file in "target/package/$package-$version.crate" "target/package/$checksum_file" "$@"; do
  name=${file##*/}
  asset=$(jq -c --arg name "$name" 'first(.assets[]? | select(.name == $name)) // null' <<< "$release")
  if [[ "$asset" == null ]]; then
    gh release upload "$RELEASE_TAG" "$file"
  elif [[ "$draft" == true && $(jq -r '.state' <<< "$asset") == starter ]]; then
    # GitHub can leave an empty starter asset after an interrupted upload.
    gh release upload "$RELEASE_TAG" "$file" --clobber
  else
    id=$(jq -er '.id' <<< "$asset")
    gh api "repos/$GH_REPO/releases/assets/$id" -H 'Accept: application/octet-stream' > "$RUNNER_TEMP/existing-asset"
    if ! cmp -s "$file" "$RUNNER_TEMP/existing-asset"; then
      # A full retry can rebuild binaries with different bytes. Refresh the
      # binaries and their manifest together while the release is a draft.
      if [[ "$draft" == true && $# -gt 0 && "$name" != "$package-$version.crate" ]]; then
        gh release upload "$RELEASE_TAG" "$file" --clobber
      else
        echo "::error::Existing release asset $name differs"
        exit 1
      fi
    fi
  fi
done
if [[ "$draft" == true ]]; then
  gh release edit "$RELEASE_TAG" --draft=false --latest="$latest"
fi
