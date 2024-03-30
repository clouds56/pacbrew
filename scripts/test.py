# %%
import json

with open("../formula.json") as f:
  data = json.load(f)

# %%
def get_bottle(item, arch=['arm64_sonoma', 'arm64_ventura', 'all']):
  files = item['bottle']['stable']['files']
  for i in arch:
    if i in files:
      return files[i]


# %%
from collections import Counter
bottles = [get_bottle(i) for i in data]
cellars = [i['cellar'] for i in bottles if i]
Counter(cellars).most_common()

# %%
for i in bottles:
  if not i: continue
  if i['cellar'] == '/opt/homebrew/Cellar':
    print(i['url'].split('/')[6])

# %%
