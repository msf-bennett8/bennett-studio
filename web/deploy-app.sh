#!/bin/bash
cd "$(dirname "$0")"
vercel --project app-bennett-studio --local-config vercel.app.json --prod
