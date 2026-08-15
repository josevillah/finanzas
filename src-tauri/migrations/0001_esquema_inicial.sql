-- Esquema inicial.
-- Convenciones: todos los montos son INTEGER (pesos chilenos, sin decimales).
-- Todas las fechas son TEXT en formato ISO 'YYYY-MM-DD'.

CREATE TABLE periodos (
  id INTEGER PRIMARY KEY,
  anio INTEGER NOT NULL,
  mes INTEGER NOT NULL,
  sueldo_liquido INTEGER NOT NULL DEFAULT 0,
  otros_ingresos INTEGER NOT NULL DEFAULT 0,
  estado TEXT NOT NULL DEFAULT 'abierto',   -- abierto | cerrado
  UNIQUE(anio, mes)
);

CREATE TABLE categorias (
  id INTEGER PRIMARY KEY,
  nombre TEXT NOT NULL,
  tipo TEXT NOT NULL,        -- fijo | variable | hormiga
  color TEXT,
  activa INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE servicios (
  id INTEGER PRIMARY KEY,
  nombre TEXT NOT NULL,
  categoria_id INTEGER REFERENCES categorias(id),
  monto_estimado INTEGER NOT NULL DEFAULT 0,
  dia_vencimiento INTEGER,
  tipo TEXT NOT NULL,        -- basico | suscripcion
  activo INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE deudas (
  id INTEGER PRIMARY KEY,
  descripcion TEXT NOT NULL,
  tipo TEXT NOT NULL,        -- compra_cuotas | credito_consumo | avance | rotativo
  institucion TEXT,
  monto_original INTEGER NOT NULL,
  tasa_mensual REAL NOT NULL DEFAULT 0,
  n_cuotas INTEGER NOT NULL,
  fecha_primera_cuota DATE NOT NULL,
  estado TEXT NOT NULL DEFAULT 'vigente',   -- vigente | pagada | repactada
  notas TEXT
);

CREATE TABLE cuotas (
  id INTEGER PRIMARY KEY,
  deuda_id INTEGER NOT NULL REFERENCES deudas(id) ON DELETE CASCADE,
  numero INTEGER NOT NULL,
  fecha_vencimiento DATE NOT NULL,
  monto INTEGER NOT NULL,
  capital INTEGER NOT NULL DEFAULT 0,
  interes INTEGER NOT NULL DEFAULT 0,
  estado TEXT NOT NULL DEFAULT 'pendiente',  -- pendiente | pagada | atrasada
  fecha_pago DATE,
  monto_pagado INTEGER,
  UNIQUE(deuda_id, numero)
);

CREATE TABLE movimientos (
  id INTEGER PRIMARY KEY,
  periodo_id INTEGER NOT NULL REFERENCES periodos(id),
  fecha DATE NOT NULL,
  monto INTEGER NOT NULL,
  tipo TEXT NOT NULL,        -- ingreso | gasto
  categoria_id INTEGER REFERENCES categorias(id),
  servicio_id INTEGER REFERENCES servicios(id),
  cuota_id INTEGER REFERENCES cuotas(id),
  medio_pago TEXT,           -- efectivo | debito | credito | transferencia
  descripcion TEXT
);

CREATE TABLE presupuestos (
  id INTEGER PRIMARY KEY,
  periodo_id INTEGER NOT NULL REFERENCES periodos(id),
  categoria_id INTEGER NOT NULL REFERENCES categorias(id),
  monto_asignado INTEGER NOT NULL,
  UNIQUE(periodo_id, categoria_id)
);

CREATE INDEX idx_cuotas_deuda ON cuotas(deuda_id);
CREATE INDEX idx_cuotas_vencimiento ON cuotas(fecha_vencimiento);
CREATE INDEX idx_cuotas_estado ON cuotas(estado);
CREATE INDEX idx_deudas_estado ON deudas(estado);
CREATE INDEX idx_movimientos_periodo ON movimientos(periodo_id);
CREATE INDEX idx_movimientos_fecha ON movimientos(fecha);
CREATE INDEX idx_movimientos_categoria ON movimientos(categoria_id);
CREATE INDEX idx_servicios_categoria ON servicios(categoria_id);
