#!/bin/bash
cd "$(dirname "$0")"

# Ensure share config is active
if [ -f "vercel.app.json" ] && [ ! -f "vercel.json" ]; then
    mv vercel.app.json vercel.json
fi

# Ensure linked to share project
if [ ! -f ".vercel/project.json" ] || ! grep -q "share-bennett-studio" ".vercel/project.json" 2>/dev/null; then
    rm -rf .vercel
    vercel link --yes --project share-bennett-studio
fi

vercel --prod
