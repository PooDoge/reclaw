import os
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))  # work from design-system/, wherever this is run
import json,sys,asyncio,os
from playwright.async_api import async_playwright
D=os.getcwd()
t=json.load(open("tokens.json"))
def css(theme):
    first=t["color"]["themes"][0]["id"]
    lines=[]
    for x in t["color"]["tokens"]:
        v=x["value"].get(theme,x["value"][first]); lines.append(f"--{x['name']}:{v};")
    for fam in ("spacing","radius","shadow","layout"):
        for x in t[fam]["tokens"]: lines.append(f"--{x['name']}:{x['value']};")
    lines.append('--font-sans:"Hanken Grotesk",system-ui,sans-serif;--font-mono:"JetBrains Mono",ui-monospace,monospace;')
    return f':root,[data-theme="{theme}"]{{{";".join(lines)}}}'
async def main():
    async with async_playwright() as p:
        b=await p.chromium.launch(executable_path="/opt/pw-browsers/chromium") if os.path.exists("/opt/pw-browsers/chromium") else await p.chromium.launch()
        for theme in ("midnight","daylight"):
            for name in sys.argv[1:]:
                src=open(f"components/{name}/preview.html").read()
                inj=f'<style>{css(theme)}</style><style>{open("components/bundle.css").read()}</style>'
                react='<script>'+open('tools/vendor/node_modules/react/umd/react.development.js').read()+'</script><script>'+open('tools/vendor/node_modules/react-dom/umd/react-dom.development.js').read()+'</script>'
                bundle=f'<script>{open("components/bundle.js").read()}</script>'
                src=src.replace('<link rel="stylesheet" href="https://fonts.googleapis.com','<link data-x href="https://fonts.googleapis.com').replace("<head>","<head>"+inj+react+bundle,1).replace("<html>",f'<html data-theme="{theme}">',1)
                pg=await b.new_page(viewport={"width":1100 if name!="Cover" else 960,"height":900})
                errs=[]; pg.on("pageerror",lambda e:errs.append(str(e))); pg.on("console",lambda m: m.type=="error" and errs.append(m.text))
                await pg.set_content(src); await pg.wait_for_timeout(1200)
                await pg.screenshot(path=f"tools/shots/{name}-{theme}.png",full_page=True)
                print(name,theme,"errors:",errs[:3])
                await pg.close()
        await b.close()
asyncio.run(main())
