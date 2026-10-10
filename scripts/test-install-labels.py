import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('install_labels', Path(__file__).with_name('install-labels.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class InstallLabelsTests(unittest.TestCase):
    def test_region_names_existing_labels_and_hacks(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            roms = root / 'Games/GBA'
            out = root / 'Labels/GBA'
            roms.mkdir(parents=True)
            out.mkdir(parents=True)
            names = ['Pokemon - Emerald Version (USA, Europe).gba',
                     'Pokemon - Fire Red Version (U) (V1.1).GBA',
                     'The Legend of Zelda - The Minish Cap (USA).gba',
                     'Pokemon - Emerald Seaglass.gba',
                     'Final Fantasy VI Advance (patched).gba',
                     'Metroid - Zero Mission (USA).zip']
            for name in names:
                (roms / name).touch()
            original = out / 'Pokemon - Emerald Version (USA, Europe).png'
            original.write_bytes(b'personal artwork')
            self.assertEqual(module.install(roms, out, True), (2, 1, 2))
            self.assertEqual(len(list(out.iterdir())), 1)
            self.assertEqual(module.install(roms, out), (2, 1, 2))
            self.assertEqual(original.read_bytes(), b'personal artwork')
            self.assertTrue((out / 'Pokemon - Fire Red Version (U) (V1.1).png').read_bytes().startswith(b'\x89PNG'))
            self.assertTrue((out / 'The Legend of Zelda - The Minish Cap (USA).png').exists())
            self.assertFalse((out / 'Pokemon - Emerald Seaglass.png').exists())
            self.assertEqual(module.install(roms, out), (0, 3, 2))


if __name__ == '__main__':
    unittest.main()
