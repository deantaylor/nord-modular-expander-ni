"""Read display names from the user's supported OS image into private local metadata.
No ROM bytes or derived label cache are bundled with the runtime or repository.
The runtime table/record layout follows G1-Emu alpha.30 g1Lib/g1knobs.h.
"""
import hashlib,struct,xml.etree.ElementTree as ET
from pathlib import Path
SUPPORTED_SHA256='d9b199f27f573fdf7cd985fbd33cb9e60893a08912ab5d1bb562c77c663c997e'
def extract(rom,modules):
    data=Path(rom).read_bytes();digest=hashlib.sha256(data).hexdigest()
    if len(data)!=524288 or digest!=SUPPORTED_SHA256:raise ValueError('Exact parameter names require the verified rack OS 3.03 image')
    tables={}
    # The OS initializes TypeTable's name pointer using a 68000 immediate
    # move at offset28 in each48-byte type record. ROM OS$c800 copies to RAM$100000.
    for pos in range(0,len(data)-6,2):
        if data[pos:pos+2] not in (b'\x45\xf9',b'\x24\x7c'):continue
        address=struct.unpack_from('>I',data,pos+2)[0];delta=address-0x1c3b00
        if delta<0 or delta%48 or delta//48>=128:continue
        for loc in range(pos+6,min(pos+240,len(data)-8),2):
            if data[loc:loc+2] in (b'\x45\xf9',b'\x24\x7c'):break
            if data[loc:loc+2]==b'\x25\x7c' and data[loc+6:loc+8]==b'\x00\x1c':
                offset=struct.unpack_from('>I',data,loc+2)[0]-0xf3800
                if 0<=offset<len(data)-12:tables[delta//48]=offset
    if len(tables)<90:raise ValueError('OS parameter name table validation failed')
    root=ET.parse(modules).getroot()
    for element in root.iter():element.tag=element.tag.split('}')[-1]
    names={}
    for module in root.iter('module'):
        kind=int(module.get('index','-1'))
        if kind not in tables:continue
        for p in module.findall('parameter'):
            index=int(p.get('index','-1'));maximum=int(p.get('maxValue','127'))
            if p.get('class','parameter')!='parameter' or not 0<=index<128 or not 0<=maximum<=127:continue
            loc=tables[kind]+index*12;record=data[loc:loc+12]
            if len(record)!=12 or record[11]!=maximum:continue
            label=record[:11].split(b'\0')[0].rstrip(b' ')
            if label and all(32<=c<127 for c in label):names[f'{kind}:{index}']={'name':label.decode('ascii'),'max':maximum}
    if len(names)<300:raise ValueError('Too few validated OS parameter names')
    return {'version':1,'romSha256':digest,'names':names}
