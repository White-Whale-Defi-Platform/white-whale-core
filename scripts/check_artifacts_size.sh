#!/bin/bash
set -e

# Maximum wasm file size
if [[ -z $1 ]]; then
  # Default max file size
  maximumSize=600
else
  maximumSize=$1
fi

echo -e "\nChecking generated wasm artifacts file size..."
for artifact in artifacts/*.wasm; do
  artifactSize=$(du -k "$artifact" | cut -f 1)
  if [ "$artifactSize" -gt $maximumSize ]; then
    echo "Artifact $(basename $artifact) file size exceeded. Found $artifactSize kB, maximum $maximumSize kB"
    exit 1
  fi
done
echo -e "All good!\n"
