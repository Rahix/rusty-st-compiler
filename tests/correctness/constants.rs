// Copyright (c) 2020 Ghaith Hachem and Mathias Rieder
use super::super::*;

#[test]
fn constant_values_used_as_initial_values() {
    #[allow(dead_code)]
    #[derive(PartialEq, Debug, Default)]
    #[repr(C)]
    struct MainType {
        i: i32,
        b: bool,
        f: f64,
    }
    // GIVEN some global variables initialized with global constants
    let src = r#"
    VAR_GLOBAL CONSTANT
        cI : DINT := 1;
        cB : BOOL := TRUE;
        cF : LREAL := 2.1543;
    END_VAR

    VAR_GLOBAL CONSTANT
        gI : DINT := cI;
        gb : BOOL := cB;
        gF : LREAL := cF;
    END_VAR

    PROGRAM main
        VAR
            i : DINT;
            b : BOOL;
            f : LREAL;
        END_VAR

        i := gI;
        b := gB;
        f := gF;
    END_PROGRAM"#;

    // WHEN the code gets executed
    let mut main = MainType::default();
    let _: i32 = compile_and_run(src, &mut main);

    //THEN we expec the globals to have the constant's values
    assert_eq!(
        main,
        MainType {
            i: 1,
            b: true,
            f: 2.1543,
        },
    );
}

#[test]
fn constant_expressions_used_as_initial_values() {
    #[allow(dead_code)]
    #[derive(PartialEq, Debug, Default)]
    #[repr(C)]
    struct MainType {
        i: i32,
        b: bool,
        f: f64,
    }
    // GIVEN some global variables initialized with global constant expressions consisting of
    // mathematical terms that only consists of CONSTANTs itself.
    let src = r#"
    VAR_GLOBAL CONSTANT
        cI : DINT := 2 * 5;
        cB : BOOL := (cI-5 = 5);
        cF : LREAL := 2.1543 + cI;
    END_VAR

    VAR_GLOBAL CONSTANT
        gI : DINT := cI;
        gb : BOOL := cB;
        gF : LREAL := cF + gI * 2;
    END_VAR

    PROGRAM main
        VAR
            i : DINT;
            b : BOOL;
            f : LREAL;
        END_VAR

        i := gI;
        b := gB;
        f := gF;
    END_PROGRAM"#;

    // WHEN the code gets executed
    let mut main = MainType::default();
    let _: i32 = compile_and_run(src, &mut main);

    //THEN we expec the globals to have the constant's values
    assert_eq!(
        main,
        MainType {
            i: 10,
            b: true,
            f: 32.1543,
        },
    );
}

#[test]
fn constant_expressions_used_in_case_statement() {
    #[allow(dead_code)]
    #[derive(PartialEq, Debug, Default)]
    #[repr(C)]
    struct MainType {
        i: i32,
        b: bool,
        f: f64,
    }
    // GIVEN some global variables initialized with global constant expressions consisting of
    // mathematical terms that only consists of CONSTANTs itself.
    let src = r#"
    VAR_GLOBAL CONSTANT
        number_1 : DINT := 2;
        number_2 : DINT := 4;
        number_3 : DINT := 8;
    END_VAR

    PROGRAM main
        VAR
            i : DINT;
            b : BOOL;
            f : LREAL;
        END_VAR

        CASE i OF
            1, 2, 3, 4:         i := 101;
            number_3:           i := 201;
            2 * number_3:       i := 301;
            number_2 + number_3:i:= 401;
            ELSE                i := 7;
    END_CASE
    END_PROGRAM"#;

    fn param(i: i32) -> MainType {
        MainType {
            i,
            b: false,
            f: 0.0,
        }
    }

    // WHEN the code gets executed
    let r1: i32 = {
        let mut p = param(777); // ELSE
        let _: i32 = compile_and_run(src, &mut p);
        p.i
    };
    let r2: i32 = {
        let mut p = param(12); // number_2 + number_3
        let _: i32 = compile_and_run(src, &mut p);
        p.i
    };
    let r3: i32 = {
        let mut p = param(16); // 2 * number_3
        let _: i32 = compile_and_run(src, &mut p);
        p.i
    };
    let r4: i32 = {
        let mut p = param(8); // number_3
        let _: i32 = compile_and_run(src, &mut p);
        p.i
    };
    let r5: i32 = {
        let mut p = param(3); // 1,2,3,4
        let _: i32 = compile_and_run(src, &mut p);
        p.i
    };
    //THEN we expect the case in reverse order
    assert_eq!((r1, r2, r3, r4, r5), (7, 401, 301, 201, 101));
}

#[test]
fn constant_expressions_used_in_array_declaration() {
    #[allow(dead_code)]
    #[derive(PartialEq, Debug, Default)]
    #[repr(C)]
    struct MainType {
        i: [i32; 10],
    }
    // GIVEN some constant and an array-declaration that defines the upper boundary
    // by calculating it via the constants
    let src = r#"
    VAR_GLOBAL CONSTANT
        TWO : DINT := 2;
        THREE : DINT := 3;
    END_VAR

    PROGRAM main
        VAR
            i : ARRAY[ 1 .. (TWO + THREE) * TWO] OF DINT;  // 1 .. 10
            j : DINT;
        END_VAR

        FOR j := 1 TO 10 BY 2 DO
            i[j] := 10*j;
            i[j+1] := 10*j + 1;
        END_FOR

    END_PROGRAM"#;

    let mut main = MainType::default();

    // WHEN the code gets executed
    let _: i32 = compile_and_run(src, &mut main);

    //THEN we expect that the array had 10 elements and was filled accordingly
    assert_eq!(main.i, [10, 11, 30, 31, 50, 51, 70, 71, 90, 91]);
}
