export type Tema = "claro" | "oscuro";

const CLAVE = "tema";

export function temaGuardado(): Tema | null {
  const t = localStorage.getItem(CLAVE);
  return t === "claro" || t === "oscuro" ? t : null;
}

export function temaActual(): Tema {
  return document.documentElement.classList.contains("dark") ? "oscuro" : "claro";
}

export function aplicarTema(tema: Tema): void {
  document.documentElement.classList.toggle("dark", tema === "oscuro");
  localStorage.setItem(CLAVE, tema);
}

export function alternarTema(): Tema {
  const siguiente: Tema = temaActual() === "oscuro" ? "claro" : "oscuro";
  aplicarTema(siguiente);
  return siguiente;
}
