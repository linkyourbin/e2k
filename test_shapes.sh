#!/bin/bash
curl -s "https://easyeda.com/api/products/C529356/components?version=6.4.19.5" | \
  python -c "import sys, json; data = json.load(sys.stdin); shapes = data['result']['dataStr']['shape']; designators = set([s.split('~')[0] for s in shapes]); print('Unique designators:', sorted(designators))"
