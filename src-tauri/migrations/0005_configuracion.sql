-- Preferencias y marcas internas de la app. Clave-valor para no tener que
-- migrar el esquema cada vez que aparece un ajuste nuevo.
CREATE TABLE configuracion (
  clave TEXT PRIMARY KEY,
  valor TEXT NOT NULL
);
