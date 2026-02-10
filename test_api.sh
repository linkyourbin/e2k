#!/bin/bash
curl -s "https://easyeda.com/api/products/C529356/components?version=6.4.19.5" | \
  python -c "import sys, json; data = json.load(sys.stdin); shapes = data['result']['dataStr']['shape']; print('Total shapes:', len(shapes)); print('C shapes:', len([s for s in shapes if s.startswith('C~')]))"
