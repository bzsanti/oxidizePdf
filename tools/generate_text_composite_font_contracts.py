#!/usr/bin/env python3
"""#666 F09/F10: full/subset CID programs, multiple FDs and Identity/stream GIDs."""
import argparse,copy,hashlib,io,json
from pathlib import Path
import fontTools
from fontTools.cffLib import CFFFontSet,FDArrayIndex,FDSelect,FontDict
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.recordingPen import RecordingPen
from fontTools.pens.t2CharStringPen import T2CharStringPen
from generate_text_symbolic_contracts import make_font
from text_contracts.fixture_pdf import assemble,cmap,stream
ROS='/Registry (Contract) /Ordering (Synthetic) /Supplement 0'

def cff_font(subset,selection_format):
    names=['.notdef','cid00017','cid00029']+([] if subset else ['cid00042'])
    widths=[500,400,700]+([] if subset else [600])
    builder=FontBuilder(1000,isTTF=False);builder.setupGlyphOrder(names);builder.setupCharacterMap({});chars={}
    for name,width in zip(names,widths):
        pen=T2CharStringPen(width,None)
        if name!='.notdef':
            pen.moveTo((50,0));pen.lineTo((width-50,0));pen.lineTo((width/2,600));pen.closePath()
        chars[name]=pen.getCharString()
    fontname=('ABCDEF+' if subset else '')+f'ContractMultiFD{selection_format}'
    builder.setupCFF(fontname,{'FullName':fontname,'FamilyName':fontname,'Weight':'Regular'},chars,{})
    builder.setupHorizontalMetrics({name:(width,0) for name,width in zip(names,widths)})
    top=builder.font['CFF '].cff.topDictIndex[0];top.ROS=('Contract','Synthetic',0);top.CIDCount=43
    array=FDArrayIndex()
    for number in [0,1]:
        fd=FontDict();fd.FontName=fontname+f'-FD{number}';fd.Private=copy.deepcopy(top.Private)
        fd.Private.BlueValues=[-10,0,590,600] if number else [-20,0,580,600]
        array.append(fd)
    select=[0,0,1]+([] if subset else [1]);top.FDArray=array;top.FDSelect=FDSelect(format=selection_format);top.FDSelect.gidArray=select
    top.CharStrings.fdArray=array;top.CharStrings.fdSelect=top.FDSelect
    for index,name in enumerate(names):top.CharStrings[name].fdSelectIndex=select[index]
    del top.Private
    raw=builder.font['CFF '].compile(builder.font);restored=CFFFontSet();restored.decompile(io.BytesIO(raw),None);decoded=restored.topDictIndex[0]
    assert decoded.charset==names and len(decoded.FDArray)==2 and decoded.FDSelect.gidArray==select
    for name,width in zip(names,widths):
        glyph=decoded.CharStrings[name];glyph.draw(RecordingPen());assert glyph.width==width
    return raw,{'name':fontname,'kind':'cff','subset':subset,'fdselect_format':selection_format,'fds':select,'charset':names,'widths':widths}

def pdf(raw,info,mode):
    ttf=info['kind']=='ttf';identity_gids=ttf and mode=='identity-gids';remap=mode=='remapped'
    cids=[2,3] if identity_gids else [17,29]
    codes=['41','42'] if remap else [f'{cid:04X}' for cid in cids]
    first_width=700 if remap else 400
    gids=bytearray(60);gids[34:36]=(2).to_bytes(2,'big');gids[58:60]=(3).to_bytes(2,'big')
    encoding='12 0 R' if remap else '/Identity-H';name=info['name']
    gidmap=('/CIDToGIDMap /Identity' if identity_gids else '/CIDToGIDMap 11 0 R') if ttf else ''
    width=f'{cids[0]} [400] {cids[1]} [700]'
    objects=[b'<< /Type /Catalog /Pages 2 0 R >>',b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
      b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',
      f'<< /Type /Font /Subtype /Type0 /BaseFont /{name} /Encoding {encoding} /DescendantFonts [6 0 R] /ToUnicode 9 0 R >>'.encode(),
      stream(f'BT /F1 10 Tf 100 700 Td <{codes[0]}> Tj 1 Tr <{codes[1]}> Tj ET'.encode()),
      f'<< /Type /Font /Subtype /CIDFontType{2 if ttf else 0} /BaseFont /{name} /CIDSystemInfo << {ROS} >> /FontDescriptor 7 0 R /DW 1000 /W [{width}] {gidmap} >>'.encode(),
      f'<< /Type /FontDescriptor /FontName /{name} /Flags 4 /FontBBox [0 0 700 600] /ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 /FontFile{2 if ttf else 3} 8 0 R >>'.encode(),
      stream(raw,f'/Length1 {len(raw)}' if ttf else '/Subtype /CIDFontType0C'),
      stream(cmap(2,'CompositeUnicode',f'1 begincodespacerange <{"00" if remap else "0000"}> <{"FF" if remap else "FFFF"}> endcodespacerange 2 beginbfchar <{codes[0]}> <0041> <{codes[1]}> <0042> endbfchar')),
      b'null',stream(bytes(gids)),
      stream(cmap(1,'CompositeRemap','1 begincodespacerange <00> <FF> endcodespacerange 2 begincidchar <41> 29 <42> 17 endcidchar',ROS),f'/Type /CMap /CMapName /CompositeRemap /CIDSystemInfo << {ROS} >> /WMode 0')]
    return assemble(objects),100+first_width/100

def generate(output):
    if fontTools.__version__!='4.60.1':raise ValueError('FontTools4.60.1 required')
    output.mkdir(parents=True,exist_ok=True);programs=[];cases=[];hashes={}
    def add(raw,info,stem,modes):
        filename=stem+('.ttf' if info['kind']=='ttf' else '.cff');(output/filename).write_bytes(raw);hashes[filename]=hashlib.sha256(raw).hexdigest();info['file']=filename;programs.append(info)
        for mode in modes:
            data,x=pdf(raw,info,mode);filename=stem+'-'+mode+'.pdf';(output/filename).write_bytes(data);hashes[filename]=hashlib.sha256(data).hexdigest()
            cases.append({'id':filename[:-4],'path':filename,'sha256':hashes[filename],'expected_text':'AB',
              'expectation_source':'Original FontTools4.60.1 full/subset programs; explicit source-code ToUnicode; CID2/3 or17/29 widths400/700. Multiple CFF FDs and CIDToGIDMap routes independently represented.',
              'expected_trace_origins':[{'text':'A','origin':[100,92]},{'text':'B','origin':[x,92]}]})
    for subset in [False,True]:
        variant='subset' if subset else 'full'
        for selection in [0,3]:
            raw,info=cff_font(subset,selection);add(raw,info,f'cff-fd{selection}-{variant}',['identity','remapped'])
        raw,info=make_font('unicode',subset);info['kind']='ttf';add(raw,info,'ttf-'+variant,['identity-gids','stream-gids','remapped'])
    (output/'readers.json').write_text(json.dumps({'schema_version':1,'issue':666,'cases':cases},indent=2)+'\n')
    (output/'provenance.json').write_text(json.dumps({'tool':'FontTools4.60.1','license':'Original project fixtures; repository MIT license','programs':programs,'sha256':hashes},indent=2)+'\n')
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('output',type=Path);generate(parser.parse_args().output)
