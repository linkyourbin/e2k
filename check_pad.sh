#!/bin/bash
curl -s "https://easyeda.com/api/products/C529356/components?version=6.4.19.5" | \
  python -c "
import sys, json
data = json.load(sys.stdin)
shapes = data['result']['packageDetail']['dataStr']['shape']
pads = [s for s in shapes if s.startswith('PAD~')]
print(f'Total pads: {len(pads)}')
if pads:
    print(f'First pad: {pads[0][:200]}')
    fields = pads[0].split('~')
    print(f'Total fields: {len(fields)}')
    for i, f in enumerate(fields[:12]):
        print(f'  [{i}]: {f[:50]}')
"
