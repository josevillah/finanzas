-- Cuentas: responder "cuánto tengo", además de "cuánto gasté".
--
-- Los saldos son declarados, no calculados desde ingresos menos gastos: un
-- saldo calculado solo cuadra si se registra absolutamente todo, y un número
-- que se desvía en silencio es peor que no tener número.

CREATE TABLE cuentas (
  id INTEGER PRIMARY KEY,
  nombre TEXT NOT NULL,
  tipo TEXT NOT NULL,        -- corriente | ahorro | informativa
  -- El CHECK acompaña al índice de abajo: si "nunca negativo" es una regla
  -- dura, que la haga cumplir SQLite. Un error al apartar falla al escribir
  -- en vez de dejar un saldo imposible.
  saldo INTEGER NOT NULL DEFAULT 0 CHECK (saldo >= 0),
  activa INTEGER NOT NULL DEFAULT 1,
  orden INTEGER NOT NULL DEFAULT 0,
  actualizado_en TEXT
);

-- Solo puede existir un saldo corriente. Garantizado por el esquema, no por
-- una validación que se pueda saltar.
CREATE UNIQUE INDEX idx_cuentas_corriente
  ON cuentas(tipo) WHERE tipo = 'corriente';

CREATE INDEX idx_cuentas_tipo ON cuentas(tipo, orden);

INSERT INTO cuentas (nombre, tipo, saldo) VALUES ('Disponible', 'corriente', 0);
