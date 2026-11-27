import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";

execFileSync(process.execPath, ["node_modules/typescript/bin/tsc", "-p", "tsconfig.json"], {
  stdio: "inherit",
});
mkdirSync("dist", { recursive: true });
for (const file of ["index.html", "style.css"]) {
  copyFileSync(`frontend/${file}`, `dist/${file}`);
}
