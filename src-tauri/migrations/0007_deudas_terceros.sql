-- Deudas de terceros: plata que otros me deben, además de lo que yo debo.

-- El DEFAULT hace el backfill solo: todas las deudas que ya existen son mías,
-- no hay ambigüedad. NOT NULL para que cualquier inserción que omita el campo
-- produzca el comportamiento de siempre.
ALTER TABLE deudas ADD COLUMN direccion TEXT NOT NULL DEFAULT 'propia';

-- Nombre de quien me debe. Obligatorio si direccion = 'tercero', NULL si es propia.
ALTER TABLE deudas ADD COLUMN deudor TEXT;

CREATE INDEX idx_deudas_direccion ON deudas(direccion);

-- Categoría donde caen los cobros. Es de tipo 'ingreso': las vistas de gasto y
-- el presupuesto la ignoran, y solo aparece al registrar un ingreso.
INSERT INTO categorias (nombre, tipo, color, activa, codigo, es_semilla)
VALUES ('Préstamos cobrados', 'ingreso', '#10b981', 1, 'cobros', 1);
