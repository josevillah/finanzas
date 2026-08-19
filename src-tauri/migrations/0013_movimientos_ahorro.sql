-- Historial de plata que entra y sale de las cuentas de ahorro.
--
-- Hasta acá una cuenta guardaba un saldo y nada más: se podía saber cuánto hay
-- apartado, pero no cuándo se apartó. Sin eso, el Resumen del mes no puede
-- explicar por qué un balance de $99.398 no se siente como $99.398 disponibles:
-- $90.000 de ese balance se fueron a un ahorro el mismo mes.
--
-- **Esta tabla es un registro, no la fuente de verdad del saldo.**
-- `cuentas.saldo` sigue siendo el valor guardado y nadie lo recalcula sumando
-- estas filas. Si algún día divergieran, manda el saldo. Los apartados
-- anteriores a esta migración no existen y no se inventan: los meses viejos
-- simplemente no muestran la línea de contexto.
--
-- Tampoco es un gasto. Un apartado no sale del patrimonio, solo cambia de
-- bolsillo, así que vive en su propia tabla y no en `movimientos`: nada de
-- esto entra en el resumen, el presupuesto, los reportes ni el disponible.

CREATE TABLE movimientos_ahorro (
  id INTEGER PRIMARY KEY,
  -- CASCADE: sin la cuenta, su historial no significa nada y quedaría
  -- huérfano ensuciando la exportación. Es también lo que hace que el
  -- reinicio de datos se lo lleve sin tener que nombrarlo.
  cuenta_id INTEGER NOT NULL REFERENCES cuentas(id) ON DELETE CASCADE,
  -- ISO 'YYYY-MM-DD'. Es el día en que se movió la plata; el corte por mes se
  -- hace por esta fecha y no por `periodo_id`, porque apartar no pertenece a
  -- un período: se puede apartar con el mes ya cerrado.
  fecha TEXT NOT NULL,
  -- Siempre positivo. La dirección la dice `tipo`, así que un monto negativo
  -- sería una segunda forma de decir lo mismo y una fuente de ambigüedad.
  monto INTEGER NOT NULL CHECK (monto > 0),
  tipo TEXT NOT NULL,        -- apartar | retirar
  nota TEXT
);

-- El uso real es "cuánto se apartó en este mes", que es un rango de fechas.
CREATE INDEX idx_mov_ahorro_fecha ON movimientos_ahorro(fecha);
