"""Exact display-label checks against the user's local supported ROM; no synth opened."""
import unittest
from pathlib import Path
from unittest.mock import patch
from rom_parameter_names import extract
ROOT=Path(__file__).resolve().parents[1]
class Names(unittest.TestCase):
 def test_supported_local_names_and_case(self):
  rom=ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'
  if not rom.exists():self.skipTest('User ROM absent')
  result=extract(rom,ROOT/'build/g1-official/nme-source/data/modules.xml');n=result['names']
  self.assertGreater(len(n),300)
  for key,value in {'7:0':'Freq coars','7:2':'KBT','20:1':'Attack','51:3':'Frequency','51:6':'Resonance','92:5':'Slope'}.items():self.assertEqual(n[key]['name'],value)
 def test_unsupported_image_is_not_guessed(self):
  with patch.object(Path,'read_bytes',return_value=b'unsupported'):
   with self.assertRaises(ValueError):extract(Path('fixture'),Path('unused'))
if __name__=='__main__':unittest.main()
