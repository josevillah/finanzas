//! Notas de propósito dentro de una cuenta de ahorro.
//!
//! Son informativas: no mueven plata, no entran en el disponible ni en el
//! patrimonio, y su suma puede quedar descuadrada respecto del saldo sin que
//! eso rompa nada.
//!
//! Lo que estos tests cuidan es la validación **asimétrica** —que nadie quede
//! encerrado sin poder corregir sus notas— y que apartar y retirar sigan sin
//! tocarlas.

use finanzas_lib::comandos::cuentas::{armar_resumen, fijar_inicial, mover, Direccion};
use finanzas_lib::comandos::notas_ahorro::{actualizar, crear, eliminar};
use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::modelos::cuenta::NuevaCuenta;
use finanzas_lib::modelos::nota_ahorro::{NotaAhorro, NuevaNota};
use finanzas_lib::repos;
use rusqlite::Connection;

fn base() -> Connection {
    let mut conn = conexion::abrir_en_memoria().expect("abrir base en memoria");
    migraciones::ejecutar(&mut conn).expect("ejecutar migraciones");
    conn
}

/// Una cuenta con plata adentro: se aparta desde el disponible, que es el
/// único camino por el que entra.
fn cuenta_con(conn: &Connection, saldo: i64) -> i64 {
    fijar_inicial(conn, saldo).unwrap();
    let id = finanzas_lib::comandos::cuentas::crear(conn, &NuevaCuenta { nombre: "Fan".into() })
        .unwrap();

    if saldo > 0 {
        mover(conn, id, saldo, Direccion::Apartar).unwrap();
    }
    id
}

fn nota(conn: &Connection, cuenta_id: i64, nombre: &str, monto: i64) -> i64 {
    crear(
        conn,
        &NuevaNota {
            cuenta_id,
            nombre: nombre.into(),
            monto,
        },
    )
    .unwrap()
}

fn notas_de(conn: &Connection, cuenta_id: i64) -> Vec<NotaAhorro> {
    repos::notas_ahorro::listar_todas(conn)
        .unwrap()
        .into_iter()
        .filter(|n| n.cuenta_id == cuenta_id)
        .collect()
}

fn suma(conn: &Connection, cuenta_id: i64) -> i64 {
    repos::notas_ahorro::suma_de_cuenta(conn, cuenta_id).unwrap()
}

// ── CRUD ─────────────────────────────────────────────────────────────────────

#[test]
fn crear_editar_y_borrar_una_nota() {
    let conn = base();
    let cuenta = cuenta_con(&conn, 100_000);

    let id = nota(&conn, cuenta, "Libros", 25_000);
    assert_eq!(notas_de(&conn, cuenta).len(), 1);

    actualizar(&conn, id, "Libros y comics", 30_000).unwrap();
    let guardada = repos::notas_ahorro::obtener(&conn, id).unwrap();
    assert_eq!(guardada.nombre, "Libros y comics");
    assert_eq!(guardada.monto, 30_000);

    eliminar(&conn, id).unwrap();
    assert!(notas_de(&conn, cuenta).is_empty());
}

#[test]
fn las_notas_se_ordenan_por_orden_de_alta() {
    let conn = base();
    let cuenta = cuenta_con(&conn, 100_000);

    nota(&conn, cuenta, "Libros", 25_000);
    nota(&conn, cuenta, "Videojuegos", 75_000);

    let nombres: Vec<String> = notas_de(&conn, cuenta)
        .into_iter()
        .map(|n| n.nombre)
        .collect();
    assert_eq!(nombres, vec!["Libros", "Videojuegos"]);
}

#[test]
fn el_nombre_vacio_y_el_monto_negativo_se_rechazan() {
    let conn = base();
    let cuenta = cuenta_con(&conn, 100_000);

    for nombre in ["", "   "] {
        assert!(crear(
            &conn,
            &NuevaNota {
                cuenta_id: cuenta,
                nombre: nombre.into(),
                monto: 1_000,
            }
        )
        .is_err());
    }

    assert!(crear(
        &conn,
        &NuevaNota {
            cuenta_id: cuenta,
            nombre: "Libros".into(),
            monto: -1,
        }
    )
    .is_err());

    assert!(notas_de(&conn, cuenta).is_empty());
}

// ── cuadratura contra el saldo ───────────────────────────────────────────────

#[test]
fn la_suma_dentro_del_saldo_se_permite() {
    let conn = base();
    let cuenta = cuenta_con(&conn, 100_000);

    nota(&conn, cuenta, "Libros", 25_000);
    nota(&conn, cuenta, "Videojuegos", 70_000);

    assert_eq!(suma(&conn, cuenta), 95_000);
}

#[test]
fn cuadrar_exacto_con_el_saldo_se_permite() {
    let conn = base();
    let cuenta = cuenta_con(&conn, 100_000);

    nota(&conn, cuenta, "Libros", 25_000);
    nota(&conn, cuenta, "Videojuegos", 75_000);

    assert_eq!(suma(&conn, cuenta), 100_000, "el borde entra");
}

#[test]
fn pasarse_del_saldo_se_rechaza_y_no_deja_nada_escrito() {
    let conn = base();
    let cuenta = cuenta_con(&conn, 100_000);
    nota(&conn, cuenta, "Libros", 90_000);

    let error = crear(
        &conn,
        &NuevaNota {
            cuenta_id: cuenta,
            nombre: "Videojuegos".into(),
            monto: 20_000,
        },
    );

    assert!(error.is_err());
    assert_eq!(notas_de(&conn, cuenta).len(), 1, "la nota nueva no se creó");
    assert_eq!(suma(&conn, cuenta), 90_000);
}

// ── la asimetría ─────────────────────────────────────────────────────────────
//
// El escenario que la justifica: retirar plata deja las notas por encima del
// saldo. Con una regla simétrica el usuario quedaría encerrado, porque
// cualquier cambio partiría de un estado ya excedido.

/// Cuenta de 100.000 con 100.000 en notas, de la que después se retiran 60.000:
/// quedan 40.000 de saldo contra 100.000 anotados.
fn cuenta_excedida(conn: &Connection) -> (i64, i64, i64) {
    let cuenta = cuenta_con(conn, 100_000);
    let libros = nota(conn, cuenta, "Libros", 25_000);
    let juegos = nota(conn, cuenta, "Videojuegos", 75_000);

    mover(conn, cuenta, 60_000, Direccion::Retirar).unwrap();

    (cuenta, libros, juegos)
}

#[test]
fn excedido_bajar_una_nota_se_permite() {
    let conn = base();
    let (cuenta, _, juegos) = cuenta_excedida(&conn);

    // Sigue excediendo (25.000 + 50.000 = 75.000 contra 40.000), pero excede
    // menos: sin esto no habría forma de empezar a corregir.
    actualizar(&conn, juegos, "Videojuegos", 50_000).unwrap();
    assert_eq!(suma(&conn, cuenta), 75_000);
}

#[test]
fn excedido_dejar_la_suma_igual_se_permite() {
    let conn = base();
    let (cuenta, libros, _) = cuenta_excedida(&conn);

    // Renombrar no mueve la suma. Bloquearlo sería la trampa que la asimetría
    // viene a evitar.
    actualizar(&conn, libros, "Libros y comics", 25_000).unwrap();

    assert_eq!(suma(&conn, cuenta), 100_000);
    assert_eq!(
        repos::notas_ahorro::obtener(&conn, libros).unwrap().nombre,
        "Libros y comics"
    );
}

#[test]
fn excedido_subir_una_nota_se_rechaza() {
    let conn = base();
    let (cuenta, _, juegos) = cuenta_excedida(&conn);

    assert!(actualizar(&conn, juegos, "Videojuegos", 75_001).is_err());
    assert_eq!(suma(&conn, cuenta), 100_000, "no cambió nada");
}

#[test]
fn excedido_agregar_otra_nota_se_rechaza() {
    let conn = base();
    let (cuenta, _, _) = cuenta_excedida(&conn);

    assert!(crear(
        &conn,
        &NuevaNota {
            cuenta_id: cuenta,
            nombre: "Ropa".into(),
            monto: 1,
        }
    )
    .is_err());
    assert_eq!(notas_de(&conn, cuenta).len(), 2);
}

#[test]
fn excedido_borrar_siempre_se_permite() {
    let conn = base();
    let (cuenta, _, juegos) = cuenta_excedida(&conn);

    eliminar(&conn, juegos).unwrap();

    assert_eq!(suma(&conn, cuenta), 25_000, "borrar solo puede bajar la suma");
}

#[test]
fn desde_excedido_se_puede_volver_a_cuadrar() {
    // El camino completo de salida, que es lo que la asimetría existe para
    // hacer posible.
    let conn = base();
    let (cuenta, libros, juegos) = cuenta_excedida(&conn);

    actualizar(&conn, juegos, "Videojuegos", 30_000).unwrap();
    actualizar(&conn, libros, "Libros", 10_000).unwrap();

    assert_eq!(suma(&conn, cuenta), 40_000);
    let saldo = repos::cuentas::obtener(&conn, cuenta).unwrap().saldo;
    assert_eq!(suma(&conn, cuenta), saldo, "cuadrado de nuevo");
}

// ── apartar y retirar no las tocan ───────────────────────────────────────────

#[test]
fn retirar_de_la_cuenta_no_modifica_las_notas() {
    let conn = base();
    let cuenta = cuenta_con(&conn, 100_000);
    nota(&conn, cuenta, "Libros", 25_000);
    nota(&conn, cuenta, "Videojuegos", 75_000);

    mover(&conn, cuenta, 60_000, Direccion::Retirar).unwrap();

    let montos: Vec<i64> = notas_de(&conn, cuenta).into_iter().map(|n| n.monto).collect();
    assert_eq!(
        montos,
        vec![25_000, 75_000],
        "ni se descuentan ni se ajustan solas: el usuario decide"
    );
}

#[test]
fn retirar_no_se_bloquea_por_las_notas() {
    let conn = base();
    let cuenta = cuenta_con(&conn, 100_000);
    nota(&conn, cuenta, "Libros", 100_000);

    // Deja la cuenta en 0 con 100.000 anotados. Es legal: solo se avisa.
    mover(&conn, cuenta, 100_000, Direccion::Retirar).unwrap();

    assert_eq!(repos::cuentas::obtener(&conn, cuenta).unwrap().saldo, 0);
    assert_eq!(suma(&conn, cuenta), 100_000);
}

#[test]
fn apartar_en_la_cuenta_no_modifica_las_notas() {
    let conn = base();
    fijar_inicial(&conn, 200_000).unwrap();
    let cuenta =
        finanzas_lib::comandos::cuentas::crear(&conn, &NuevaCuenta { nombre: "Fan".into() })
            .unwrap();
    mover(&conn, cuenta, 100_000, Direccion::Apartar).unwrap();
    nota(&conn, cuenta, "Libros", 25_000);

    mover(&conn, cuenta, 50_000, Direccion::Apartar).unwrap();

    let montos: Vec<i64> = notas_de(&conn, cuenta).into_iter().map(|n| n.monto).collect();
    assert_eq!(montos, vec![25_000], "entra plata, las notas siguen igual");
}

// ── independencia de los cálculos ────────────────────────────────────────────

#[test]
fn las_notas_no_mueven_el_disponible_ni_el_patrimonio() {
    let conn = base();
    let cuenta = cuenta_con(&conn, 100_000);

    let antes = armar_resumen(&conn).unwrap();

    nota(&conn, cuenta, "Libros", 25_000);
    nota(&conn, cuenta, "Videojuegos", 75_000);

    let despues = armar_resumen(&conn).unwrap();

    assert_eq!(despues.disponible, antes.disponible);
    assert_eq!(despues.patrimonio, antes.patrimonio);
    assert_eq!(despues.total_ahorrado, antes.total_ahorrado);
    assert_eq!(despues.desglose.apartado, antes.desglose.apartado);
}

#[test]
fn el_resumen_trae_las_notas_con_su_cuadratura() {
    let conn = base();
    let cuenta = cuenta_con(&conn, 100_000);
    nota(&conn, cuenta, "Libros", 25_000);

    let resumen = armar_resumen(&conn).unwrap();
    let fila = resumen.ahorros.iter().find(|a| a.cuenta.id == cuenta).unwrap();

    assert_eq!(fila.notas.len(), 1);
    assert_eq!(fila.total_notas, 25_000);
    assert_eq!(fila.sin_asignar, 75_000, "positivo: queda plata sin anotar");
}

#[test]
fn excedida_el_resumen_lo_informa_en_negativo() {
    let conn = base();
    let (cuenta, _, _) = cuenta_excedida(&conn);

    let resumen = armar_resumen(&conn).unwrap();
    let fila = resumen.ahorros.iter().find(|a| a.cuenta.id == cuenta).unwrap();

    assert_eq!(fila.total_notas, 100_000);
    assert_eq!(fila.sin_asignar, -60_000, "40.000 de saldo, 100.000 anotados");
}

#[test]
fn una_cuenta_sin_notas_se_comporta_igual_que_antes() {
    let conn = base();
    let cuenta = cuenta_con(&conn, 100_000);

    let resumen = armar_resumen(&conn).unwrap();
    let fila = resumen.ahorros.iter().find(|a| a.cuenta.id == cuenta).unwrap();

    assert!(fila.notas.is_empty());
    assert_eq!(fila.total_notas, 0);
    assert_eq!(fila.sin_asignar, fila.cuenta.saldo);
    assert_eq!(resumen.disponible, 0);
    assert_eq!(resumen.patrimonio, 100_000);
}

// ── el vínculo con la cuenta ─────────────────────────────────────────────────

#[test]
fn borrar_la_cuenta_borra_sus_notas() {
    let conn = base();
    let cuenta = cuenta_con(&conn, 100_000);
    nota(&conn, cuenta, "Libros", 25_000);
    nota(&conn, cuenta, "Videojuegos", 75_000);

    // Eliminar exige la cuenta vacía; las notas quedan igual hasta el final.
    mover(&conn, cuenta, 100_000, Direccion::Retirar).unwrap();
    finanzas_lib::comandos::cuentas::eliminar(&conn, cuenta).unwrap();

    assert!(
        repos::notas_ahorro::listar_todas(&conn).unwrap().is_empty(),
        "el ON DELETE CASCADE no puede dejar notas huérfanas"
    );
}

#[test]
fn las_notas_de_una_cuenta_no_afectan_a_otra() {
    let conn = base();
    fijar_inicial(&conn, 200_000).unwrap();

    let fan =
        finanzas_lib::comandos::cuentas::crear(&conn, &NuevaCuenta { nombre: "Fan".into() })
            .unwrap();
    let viaje =
        finanzas_lib::comandos::cuentas::crear(&conn, &NuevaCuenta { nombre: "Viaje".into() })
            .unwrap();

    mover(&conn, fan, 100_000, Direccion::Apartar).unwrap();
    mover(&conn, viaje, 100_000, Direccion::Apartar).unwrap();

    nota(&conn, fan, "Libros", 100_000);

    // La cuenta de al lado tiene su propio techo, no el de la vecina.
    nota(&conn, viaje, "Pasajes", 100_000);

    assert_eq!(suma(&conn, fan), 100_000);
    assert_eq!(suma(&conn, viaje), 100_000);
}
