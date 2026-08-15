-- Los servicios recurrentes ahora sí generan su gasto del mes.

-- Desde cuándo existe el servicio. Acota la generación: nunca se crean gastos
-- en meses anteriores al alta, así no se contaminan períodos ya cerrados o de
-- años previos.
ALTER TABLE servicios ADD COLUMN fecha_alta DATE;
UPDATE servicios SET fecha_alta = date('now') WHERE fecha_alta IS NULL;

-- Marca el gasto que puso el sistema con el monto estimado y que todavía no
-- confirma el usuario. Al cambiarle el precio o editarlo, pasa a 0.
-- Sin esta marca la comparación estimado vs. real sería siempre cero.
ALTER TABLE movimientos ADD COLUMN es_estimado INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_movimientos_estimado ON movimientos(es_estimado);

-- Un servicio genera a lo más un gasto estimado por período.
CREATE UNIQUE INDEX idx_movimientos_servicio_estimado
  ON movimientos(periodo_id, servicio_id)
  WHERE servicio_id IS NOT NULL AND es_estimado = 1;
