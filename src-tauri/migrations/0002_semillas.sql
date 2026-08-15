-- Categorías base. Sirven de punto de partida; son editables desde la app.

INSERT INTO categorias (nombre, tipo, color) VALUES
  ('Arriendo / Dividendo', 'fijo',     '#6366f1'),
  ('Servicios básicos',    'fijo',     '#0ea5e9'),
  ('Internet y telefonía', 'fijo',     '#14b8a6'),
  ('Suscripciones',        'fijo',     '#8b5cf6'),
  ('Deudas y créditos',    'fijo',     '#ef4444'),
  ('Supermercado',         'variable', '#22c55e'),
  ('Transporte',           'variable', '#f59e0b'),
  ('Salud',                'variable', '#06b6d4'),
  ('Educación',            'variable', '#3b82f6'),
  ('Hogar',                'variable', '#a855f7'),
  ('Café y snacks',        'hormiga',  '#f97316'),
  ('Delivery',             'hormiga',  '#e11d48'),
  ('Salidas y carrete',    'hormiga',  '#d946ef'),
  ('Compras impulsivas',   'hormiga',  '#fb7185');
