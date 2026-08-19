-- Limpieza de los gastos estimados que quedaron en meses que todavía no
-- llegaron.
--
-- Hasta esta versión, `generar_gastos_servicios` aceptaba cualquier mes y la
-- pantalla los generaba sola al entrar. Con el selector permitiendo avanzar
-- mientras su rango no había cargado, bastaba un click en la flecha para
-- materializar los estimados de un mes futuro. Esos movimientos descontaban
-- disponible por plata que no había salido, y como el selector no alcanza los
-- meses futuros, no había forma de verlos ni borrarlos desde la aplicación.
--
-- Es la primera migración de datos del proyecto. Va acá y no en un script
-- suelto porque la app está instalada en varios equipos y ahí nadie puede
-- correr SQL a mano: la limpieza tiene que viajar con la actualización.
--
-- El criterio es deliberadamente conservador. Ante la duda sobre si un
-- movimiento lo escribió una persona o lo generó el sistema, no se toca:
-- mejor que sobre un fantasma a que se borre algo que alguien registró.

-- `es_estimado = 1` es un marcador exclusivo del generador automático:
-- `insertar_estimado_servicio` es el único que lo escribe, la activación
-- manual nace en 0, y editar el movimiento o cambiarle el precio lo bajan a 0.
-- Una fila que sigue en 1 la creó el sistema y no la tocó nadie.
--
-- El resto de las condiciones son todas restrictivas:
--   cuota_id IS NULL        -> jamás el pago de una cuota
--   servicio_id IS NOT NULL -> todo estimado automático nace atado a un servicio
--   tipo = 'gasto'          -> el generador no produce ingresos
--   estado = 'abierto'      -> un mes cerrado lo cerró alguien a conciencia
--   > mes actual            -> estrictamente futuro; el mes en curso no se toca,
--                              ahí los estimados son legítimos
--
-- El mes actual se calcula con 'localtime' y no en UTC, para coincidir con
-- `fechas::hoy()`, que usa la hora local. Ese "ahora" es el momento en que
-- cada equipo instala esta versión.
DELETE FROM movimientos
WHERE es_estimado = 1
  AND cuota_id IS NULL
  AND servicio_id IS NOT NULL
  AND tipo = 'gasto'
  AND periodo_id IN (
        SELECT id FROM periodos
         WHERE estado = 'abierto'
           AND (anio * 12 + mes) >
               CAST(strftime('%Y', 'now', 'localtime') AS INTEGER) * 12
             + CAST(strftime('%m', 'now', 'localtime') AS INTEGER)
      );

-- El período futuro que quedó sin nada adentro se va con ellos: existía solo
-- porque `obtener_o_crear` lo creó al pasar por ahí, y dejarlo haría que el
-- mes siguiera figurando como un mes con datos.
--
-- `movimientos` y `presupuestos` son las únicas dos tablas que referencian
-- `periodos`, así que estas dos subconsultas cubren toda la integridad
-- referencial. Un período futuro con sueldo declarado, con presupuesto o con
-- cualquier movimiento sobreviviente queda intacto.
DELETE FROM periodos
WHERE estado = 'abierto'
  AND sueldo_liquido = 0
  AND otros_ingresos = 0
  AND (anio * 12 + mes) >
      CAST(strftime('%Y', 'now', 'localtime') AS INTEGER) * 12
    + CAST(strftime('%m', 'now', 'localtime') AS INTEGER)
  AND id NOT IN (SELECT periodo_id FROM movimientos)
  AND id NOT IN (SELECT periodo_id FROM presupuestos);
