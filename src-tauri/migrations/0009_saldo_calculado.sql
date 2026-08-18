-- El saldo disponible deja de ser un valor declarado y pasa a calcularse:
--
--   saldo_inicial + ingresos - gastos = patrimonio
--   patrimonio - ahorros apartados    = disponible
--
-- Con eso desaparecen las cuentas de tipo 'corriente' (el disponible ya no es
-- una fila, es una cuenta aritmética) y las 'informativa' (eran una anotación
-- paralela que había que cuadrar a mano). La tabla queda solo con ahorros, así
-- que la columna `tipo` pierde sentido y se va con ella el índice único
-- parcial que garantizaba una sola corriente.
--
-- Se recrea la tabla en vez de alterarla: los datos de la versión anterior son
-- saldos declarados bajo otro modelo y no tienen traducción a este.
DROP TABLE cuentas;

CREATE TABLE cuentas (
  id INTEGER PRIMARY KEY,
  nombre TEXT NOT NULL,
  -- Un ahorro en rojo no significa nada: la plata se aparta desde el
  -- disponible o no se aparta.
  saldo INTEGER NOT NULL DEFAULT 0 CHECK (saldo >= 0),
  activa INTEGER NOT NULL DEFAULT 1,
  orden INTEGER NOT NULL DEFAULT 0,
  actualizado_en TEXT
);

CREATE INDEX idx_cuentas_orden ON cuentas(orden, nombre);

-- Lo que el usuario tenía antes de empezar a usar la app. Es el único número
-- que ajusta a mano para que el disponible calce con su banco.
--
-- A propósito NO es un movimiento de ingreso: uno ficticio inflaría el resumen
-- de ese mes y torcería el reporte de evolución para siempre.
INSERT INTO configuracion (clave, valor) VALUES ('saldo_inicial', '0')
  ON CONFLICT(clave) DO NOTHING;
