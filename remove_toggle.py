import re

filepath = 'src/plugins/ui-default/Sidebar.vue'
with open(filepath, 'r', encoding='utf-8') as f:
    text = f.read()

# Remove the button that uses playerStore.isDarkMode
text = re.sub(r'<button @click="playerStore\.isDarkMode = !playerStore\.isDarkMode".*?</button>', '', text, flags=re.DOTALL)

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(text)
