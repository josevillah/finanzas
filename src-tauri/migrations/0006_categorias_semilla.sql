-- Marca cuáles categorías vienen de fábrica.
--
-- Hace falta para el reinicio de datos: sin el flag, una categoría semilla y
-- una creada por el usuario son indistinguibles en la base. Comparar por
-- nombre no sirve, porque el usuario puede renombrarlas y entonces su semilla
-- renombrada parecería creada por él.

ALTER TABLE categorias ADD COLUMN es_semilla INTEGER NOT NULL DEFAULT 0;

-- Relleno por única vez contra los nombres de `0002_semillas.sql`. De acá en
-- adelante el flag manda y los renombres dejan de importar.
UPDATE categorias SET es_semilla = 1 WHERE nombre IN (
  'Arriendo / Dividendo',
  'Servicios básicos',
  'Internet y telefonía',
  'Suscripciones',
  'Deudas y créditos',
  'Supermercado',
  'Transporte',
  'Salud',
  'Educación',
  'Hogar',
  'Café y snacks',
  'Delivery',
  'Salidas y carrete',
  'Compras impulsivas'
);

-- Una base que ya tenía la categoría de deudas renombrada igual debe contarla
-- como semilla: el código la ubica por su código estable, no por el nombre.
UPDATE categorias SET es_semilla = 1 WHERE codigo IS NOT NULL;

CREATE INDEX idx_categorias_semilla ON categorias(es_semilla);
