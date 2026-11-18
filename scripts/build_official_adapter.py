"""Build only an external protocol adapter; never rebuild or alter the official VST."""
import argparse,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
NME_PIN='0ded1bb77ca2431b7ff52a8b55076430d85ac48d'
JUCE_PIN='29396c22c93392d6738e021b83196283d6e4d850'
def checkout(path,url,pin):
    if not path.exists():
        subprocess.run(['git','init',str(path)],check=True)
        subprocess.run(['git','-C',str(path),'fetch','--depth','1',url,pin],check=True)
        subprocess.run(['git','-C',str(path),'checkout','--detach','FETCH_HEAD'],check=True)
    assert subprocess.check_output(['git','-C',str(path),'rev-parse','HEAD'],text=True).strip()==pin,f'Unexpected source revision: {path}'
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--nme-dir',type=Path,default=ROOT/'build/g1-official/nme-source');parser.add_argument('--juce-dir',type=Path,default=ROOT/'build/g1-official/juce-source');args=parser.parse_args()
    checkout(args.nme_dir,'https://github.com/animatek/Animatek-NME.git',NME_PIN);checkout(args.juce_dir,'https://github.com/juce-framework/JUCE.git',JUCE_PIN)
    build=ROOT/'build/g1-official/adapter-build'
    subprocess.run(['cmake','-S',str(ROOT/'integrations/g1-official'),'-B',str(build),'-DCMAKE_BUILD_TYPE=Release','-DCMAKE_OSX_ARCHITECTURES=arm64',f'-DNME_SOURCE={args.nme_dir.resolve()}',f'-DJUCE_SOURCE={args.juce_dir.resolve()}'],check=True)
    subprocess.run(['cmake','--build',str(build),'--target','g1keyboardadapter','--parallel','4'],check=True)
if __name__=='__main__':main()
