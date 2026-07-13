#!/bin/bash
cd "$(dirname "$0")"
vercel --project share-bennett-studio --local-config vercel.share.json --prod
