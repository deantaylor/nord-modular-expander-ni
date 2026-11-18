"""Rasterize the supplied 2:1 Eastlab SVG using macOS Quick Look and Pillow."""
from pathlib import Path
import subprocess, tempfile
from PIL import Image
ROOT = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='eastlab-logo-') as folder:
    source = ROOT / 'screens/assets/eastlab-logo-dark.svg'
    subprocess.run(['qlmanage', '-t', '-s', '1920', '-o', folder, str(source)], check=True, capture_output=True)
    image = Image.open(Path(folder) / (source.name + '.png')).convert('RGB')
    width, height = image.size
    top = (height - width // 2) // 2
    artwork = image.crop((0, top, width, top + width // 2)).resize((288, 144), Image.Resampling.LANCZOS)
    frame = Image.new('RGB', (480, 272), '#141414')
    frame.paste(artwork, (96, 64))
    frame.save(ROOT / 'vendor/encdr/assets/eastlab-loading.png')
