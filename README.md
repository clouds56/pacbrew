# Download cache
1. wget -S --header="accept-encoding: gzip" https://formulae.brew.sh/api/formula.json
2. mv formula.json formula.json.gz && gzip -d formula.json.gz
