import json

with open("errors.json", "r") as f, open("errors_out.txt", "w") as out:
    for line in f:
        try:
            data = json.loads(line)
            if "message" in data:
                out.write(data["message"]["rendered"] + "\n")
        except:
            pass
