/**
 * Sincroniza la versión de package.json hacia Cargo.toml y tauri.conf.json.
 *
 * Los tres archivos tienen que decir lo mismo: el updater compara la versión
 * del binario (Cargo.toml) contra la del release. Si quedan desfasados, o no
 * ofrece la actualización, o la ofrece en bucle porque tras instalar sigue
 * viéndose más viejo.
 *
 *   node scripts/version.mjs          -> sincroniza con package.json
 *   node scripts/version.mjs 0.2.0    -> fija esa versión en los tres
 */

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const raiz = join(dirname(fileURLToPath(import.meta.url)), "..");

const rutaPackage = join(raiz, "package.json");
const rutaCargo = join(raiz, "src-tauri", "Cargo.toml");
const rutaTauri = join(raiz, "src-tauri", "tauri.conf.json");

const paquete = JSON.parse(readFileSync(rutaPackage, "utf8"));
const version = process.argv[2] ?? paquete.version;

if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`Versión inválida: "${version}". Se espera MAYOR.MENOR.PARCHE, por ejemplo 0.2.0.`);
  process.exit(1);
}

// Cargo.toml: solo la primera aparición, que es la del paquete. Las versiones
// de las dependencias vienen después y no se tocan.
// El `\r?` es necesario: los archivos están con saltos CRLF y sin él `$` no
// llega a calzar.
const cargo = readFileSync(rutaCargo, "utf8");
const cargoNuevo = cargo.replace(/^version = "[^"]*"\r?$/m, `version = "${version}"`);

if (cargo === cargoNuevo && !cargo.includes(`version = "${version}"`)) {
  console.error("No se encontró la línea de versión en Cargo.toml.");
  process.exit(1);
}

const tauri = JSON.parse(readFileSync(rutaTauri, "utf8"));

// Recién acá se escribe: si algo falla arriba, los tres archivos quedan como
// estaban en vez de a medio actualizar.
paquete.version = version;
tauri.version = version;

writeFileSync(rutaPackage, `${JSON.stringify(paquete, null, 2)}\n`);
writeFileSync(rutaCargo, cargoNuevo);
writeFileSync(rutaTauri, `${JSON.stringify(tauri, null, 2)}\n`);

console.log(`Versión ${version} aplicada en package.json, Cargo.toml y tauri.conf.json.`);
console.log("Siguiente paso:");
console.log(`  git commit -am "v${version}" && git tag v${version} && git push --follow-tags`);
