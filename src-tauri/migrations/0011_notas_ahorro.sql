-- Notas de propósito dentro de una cuenta de ahorro.
--
-- Sirven para desglosar un saldo que en la cabeza del usuario ya está dividido:
-- la cuenta "Fan" tiene $100.000, y de esos $25.000 son para libros y $75.000
-- para videojuegos.
--
-- Son informativas y opcionales. No entran en ningún cálculo del sistema —ni el
-- disponible, ni el patrimonio, ni los reportes— y una cuenta sin notas se
-- comporta exactamente igual que antes de esta tabla. La plata sigue moviéndose
-- solo con apartar y retirar, que no las tocan.
CREATE TABLE notas_ahorro (
  id INTEGER PRIMARY KEY,
  -- Borrar la cuenta se lleva sus notas: sin la cuenta no significan nada, y
  -- dejarlas huérfanas ensuciaría la exportación con filas que no se pueden
  -- atribuir a nada.
  cuenta_id INTEGER NOT NULL REFERENCES cuentas(id) ON DELETE CASCADE,
  nombre TEXT NOT NULL,
  -- Una nota en rojo no significa nada: lo que se anota es cuánto de lo que hay
  -- está reservado para algo.
  monto INTEGER NOT NULL DEFAULT 0 CHECK (monto >= 0),
  orden INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_notas_ahorro_cuenta ON notas_ahorro(cuenta_id, orden);
