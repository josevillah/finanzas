-- Metas: "cuánto necesito para esto" y cuánto llevo.
--
-- Una meta no mueve plata. No es un movimiento, no toca el disponible ni el
-- patrimonio: es una etiqueta sobre plata que ya está en algún ahorro, o un
-- número de referencia si todavía no hay nada apartado. Por eso vive en su
-- propia tabla y ningún cálculo de saldos la consulta.
--
-- La 0011 (notas de propósito en cuentas de ahorro) se desarrolló en paralelo
-- y ya está en main: las migraciones quedan consecutivas, 11 y después 12.

CREATE TABLE metas (
  id INTEGER PRIMARY KEY,
  nombre TEXT NOT NULL,
  -- Una meta de $0 no es una meta. El CHECK lo hace cumplir el esquema y no
  -- solo la validación del comando, que se puede saltar.
  monto_objetivo INTEGER NOT NULL CHECK (monto_objetivo > 0),
  -- Opcional: sin cuenta vinculada la meta es solo referencia de cuánto
  -- necesito, sin barra de avance.
  --
  -- ON DELETE SET NULL y no CASCADE: borrar una cuenta de ahorro no puede
  -- borrar el objetivo. Se pierde el progreso, no la meta.
  cuenta_id INTEGER REFERENCES cuentas(id) ON DELETE SET NULL,
  -- Menor es más prioritaria. Cuando varias metas comparten cuenta, este
  -- orden decide quién consume el saldo primero.
  prioridad INTEGER NOT NULL DEFAULT 0,
  fecha_objetivo TEXT,
  estado TEXT NOT NULL DEFAULT 'activa',   -- activa | cumplida | archivada
  notas TEXT,
  creada_en TEXT NOT NULL
);

-- El orden en que se listan y se reparte el saldo.
CREATE INDEX idx_metas_orden ON metas(estado, prioridad, id);

-- El reparto por cuenta recorre las metas activas de una cuenta en orden.
CREATE INDEX idx_metas_cuenta ON metas(cuenta_id, prioridad, id);
