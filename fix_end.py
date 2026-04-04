with open("src/engine/effects.rs", "r") as f:
    lines = f.readlines()

# find the FIRST valid function block and truncate there, not the last one!
target = "        [y].into()\n"
for i in range(len(lines)):
    if lines[i] == target and lines[i+1] == "    }\n" and lines[i+2] == "}\n":
        lines = lines[:i+3]
        break

with open("src/engine/effects.rs", "w") as f:
    f.writelines(lines)
