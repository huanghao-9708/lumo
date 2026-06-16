import os
import re

simple_dir = r"d:\code\rust\lumo\src\themes\simple"

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # If the file imports from stores, api, composables, utils, we need to add one more `../`
    # Match relative imports that go up to src/
    # E.g. import { ... } from '../stores/player' -> import { ... } from '../../stores/player'
    # import { ... } from '../../stores/player' -> import { ... } from '../../../stores/player'
    
    # regex for import strings that start with ../ or ../../
    # We replace from longest to shortest to avoid double replacing.
    
    replacements = [
        (r'\.\./\.\./\.\./(stores|api|composables|utils)', r'../../../../\1'),
        (r'\.\./\.\./(stores|api|composables|utils)', r'../../../\1'),
        (r'\.\./(stores|api|composables|utils)', r'../../\1'),
    ]
    
    for old, new in replacements:
        content = re.sub(old, new, content)

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

for root, _, files in os.walk(simple_dir):
    for file in files:
        if file.endswith('.vue') or file.endswith('.ts'):
            process_file(os.path.join(root, file))

print("Imports updated for themes/simple")
