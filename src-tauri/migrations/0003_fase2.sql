-- Fase 2: gastos, servicios recurrentes y enlace cuota -> movimiento.

-- Referencia estable a las categorías que el código necesita ubicar por sí
-- mismo. El nombre visible sigue siendo editable por el usuario.
ALTER TABLE categorias ADD COLUMN codigo TEXT;

CREATE UNIQUE INDEX idx_categorias_codigo
  ON categorias(codigo) WHERE codigo IS NOT NULL;

UPDATE categorias SET codigo = 'deudas' WHERE nombre = 'Deudas y créditos';

-- Un pago de cuota genera exactamente un movimiento: el índice lo garantiza
-- aunque algo falle a mitad de camino.
CREATE UNIQUE INDEX idx_movimientos_cuota
  ON movimientos(cuota_id) WHERE cuota_id IS NOT NULL;

CREATE INDEX idx_movimientos_servicio ON movimientos(servicio_id);
CREATE INDEX idx_movimientos_tipo ON movimientos(tipo);
