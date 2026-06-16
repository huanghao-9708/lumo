import os
import re

def process_file(filepath, replacements):
    with open(filepath, 'r', encoding='utf-8') as f:
        text = f.read()
    
    for old, new in replacements:
        # Only replace if the new value isn't already there to prevent doubling
        if new not in text:
            # use regex to replace as whole word if possible, or simple replace
            # since we're replacing class names, a simple string replace usually works
            text = text.replace(old, new)
            
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(text)

# ui-default replacements
ui_default_replacements = [
    # background
    ('bg-white', 'bg-white dark:bg-[#121212]'),
    ('bg-gray-50', 'bg-gray-50 dark:bg-[#1e1e1e]'),
    ('bg-gray-100', 'bg-gray-100 dark:bg-[#2a2a2a]'),
    ('bg-gray-200', 'bg-gray-100 dark:bg-[#333333]'), # softened
    ('bg-brand-orange-bg', 'bg-brand-orange-bg dark:bg-brand-orange/20'),
    
    # borders
    ('border-gray-200', 'border-gray-100 dark:border-gray-800'), # softened in light mode
    ('border-gray-100', 'border-gray-50 dark:border-gray-800'),
    
    # text
    ('text-gray-900', 'text-gray-900 dark:text-gray-100'),
    ('text-gray-800', 'text-gray-800 dark:text-gray-200'),
    ('text-gray-600', 'text-gray-600 dark:text-gray-300'),
    ('text-gray-500', 'text-gray-500 dark:text-gray-400'),
    ('text-[#333333]', 'text-[#333333] dark:text-[#f3f4f6]'),
    ('text-[#888888]', 'text-[#888888] dark:text-[#9ca3af]'),
    
    # hover
    ('hover:bg-gray-100', 'hover:bg-gray-100 dark:hover:bg-gray-800'),
    ('hover:bg-gray-50', 'hover:bg-gray-50 dark:hover:bg-gray-800/50'),
    ('hover:bg-white', 'hover:bg-white dark:hover:bg-[#2a2a2a]'),
    ('hover:text-gray-900', 'hover:text-gray-900 dark:hover:text-white'),
    ('hover:text-gray-800', 'hover:text-gray-800 dark:hover:text-gray-100'),
    ('hover:text-gray-700', 'hover:text-gray-700 dark:hover:text-gray-200'),
]

# ui-simple replacements
ui_simple_replacements = [
    # backgrounds
    ('bg-[#fdfcf9]', 'bg-[#fdfcf9] dark:bg-[#1a1a1a]'),
    ('bg-[#eae8e1]', 'bg-[#eae8e1] dark:bg-[#333]'),
    ('bg-[#1a1a1a]', 'bg-[#1a1a1a] dark:bg-[#fdfcf9]'),
    
    # borders
    ('border-[#1a1a1a]', 'border-[#1a1a1a] dark:border-[#fdfcf9]'),
    ('border-[#dcdad1]', 'border-[#dcdad1] dark:border-[#333]'),
    ('border-transparent', 'border-transparent dark:border-transparent'),
    
    # text
    ('text-[#1a1a1a]', 'text-[#1a1a1a] dark:text-[#fdfcf9]'),
    ('text-[#fdfcf9]', 'text-[#fdfcf9] dark:text-[#1a1a1a]'),
    ('text-[#333]', 'text-[#333] dark:text-[#d1d5db]'),
    ('text-[#888]', 'text-[#888] dark:text-[#9ca3af]'),
    ('text-[#999]', 'text-[#999] dark:text-[#6b7280]'),
    
    # hover
    ('hover:bg-[#eae8e1]', 'hover:bg-[#eae8e1] dark:hover:bg-[#333]'),
    ('hover:bg-[#1a1a1a]', 'hover:bg-[#1a1a1a] dark:hover:bg-[#fdfcf9]'),
    ('hover:text-[#1a1a1a]', 'hover:text-[#1a1a1a] dark:hover:text-white'),
    ('hover:text-black', 'hover:text-black dark:hover:text-white'),
]

base_dir = 'src/plugins'

for ui_dir, replacements in [('ui-default', ui_default_replacements), ('ui-simple', ui_simple_replacements)]:
    folder = os.path.join(base_dir, ui_dir)
    for root, dirs, files in os.walk(folder):
        for file in files:
            if file.endswith('.vue'):
                filepath = os.path.join(root, file)
                process_file(filepath, replacements)
                
print("Dark mode classes added.")
