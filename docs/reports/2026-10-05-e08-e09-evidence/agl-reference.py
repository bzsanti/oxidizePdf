"""Recheck the independent literal AGL expectations; no product tables."""
import json, sys
from pathlib import Path
sys.path.insert(0,'target/issue666-fonttools')
import fontTools
from fontTools.agl import toUnicode
assert fontTools.__version__=='4.60.1'
data=json.loads(Path(__file__).with_suffix('.json').read_text())
for name,expected in data['cases']:
    assert toUnicode(name)==expected,(name,toUnicode(name))
name,expected=data['zapf']
assert toUnicode(name,isZapfDingbats=True)==expected
print('Seven independent FontTools4.60.1 AGL controls pass')
