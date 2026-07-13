#!/bin/bash
cd "$(dirname "$0")"

# Save share config if present
if [ -f "vercel.json" ] && [ ! -f "vercel.share.json" ]; then
    cp vercel.json vercel.share.json
fi

# Swap to app config
cp vercel.app.json vercel.json

# Remove old link, create new app project link
rm -rf .vercel
vercel link --yes --project app-bennett-studio

vercel --prod
