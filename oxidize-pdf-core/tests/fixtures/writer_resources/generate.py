"""Regenerate the CJK subsetting fixture with fontTools 4.60.1.
Usage: python3 generate.py SOURCE_OTF OUTPUT_OTF
"""
import hashlib
import sys
from pathlib import Path
import fontTools
from fontTools import subset
from fontTools.ttLib import TTFont

source, destination = map(Path, sys.argv[1:])
assert fontTools.__version__ == '4.60.1'
assert hashlib.sha256(source.read_bytes()).hexdigest() == 'f1d8611151880c6c336aabeac4640ef434fa13cbfbf1ffe82d0a71b2a5637256'
font = TTFont(source, recalcTimestamp=False)
options = subset.Options()
options.hinting = False
options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14, 16, 17, 18]
options.name_languages = ['*']
subsetter = subset.Subsetter(options=options)
# Keep enough real glyphs to exercise automatic CFF subsetting (>100 KB),
# as the original full-size CJK font does; also include the four test characters.
subsetter.populate(unicodes=set(range(0x4E00, 0x5200)) | {ord(c) for c in '中文测试'})
subsetter.subset(font)
# The modified fixture does not use the reserved font name "Source".
renames = {1: 'WriterCjkTest', 2: 'Regular', 3: 'WriterCjkTest-Regular',
           4: 'WriterCjkTest Regular', 6: 'WriterCjkTest-Regular',
           16: 'WriterCjkTest', 17: 'Regular', 18: 'WriterCjkTest Regular'}
for record in list(font['name'].names):
    if record.nameID in renames:
        font['name'].setName(renames[record.nameID], record.nameID,
                             record.platformID, record.platEncID, record.langID)
cff = font['CFF '].cff
cff.fontNames = ['WriterCjkTest-Regular']
for top in cff.topDictIndex:
    top.FamilyName = 'WriterCjkTest'
    top.FullName = 'WriterCjkTest Regular'
font.save(destination)
assert destination.stat().st_size > 100_000, 'fixture must exercise automatic subsetting'
print(hashlib.sha256(destination.read_bytes()).hexdigest())
