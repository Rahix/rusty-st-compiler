// Copyright (c) 2020 Ghaith Hachem and Mathias Rieder
use super::super::*;

#[derive(Default)]
#[repr(C)]
struct MainType {
    a: f32,
    b: f32,
    c: f64,
    d: f64,
}

#[test]
fn real_negation() {
    let function = "
            PROGRAM main
            VAR
                a,b : REAL;
                c,d : LREAL;
            END_VAR
                a := -2.0;
                b := -a;
                c := -3.0;
                d := -c;
            END_PROGRAM
    ";
    let mut maintype = MainType::default();
    let _: i32 = compile_and_run(function, &mut maintype);
    assert_eq!(-2.0, maintype.a);
    assert_eq!(2.0, maintype.b);
    assert_eq!(-3.0, maintype.c);
    assert_eq!(3.0, maintype.d);
}

#[test]
fn equal_comparison_with_arbitrary_datatypes() {
    #[repr(C)]
    struct Main {
        results: [i32; 2],
    }

    let mut main = Main { results: [0, 0] };

    let function = "
            FUNCTION STRING_EQUAL : BOOL
                VAR_INPUT a,b : STRING; END_VAR
                STRING_EQUAL := TRUE;
            END_FUNCTION

            PROGRAM main 
            VAR
                result1 : DINT;
                result2 : DINT;
            END_VAR
            VAR_TEMP
                a,b : STRING[1];
            END_VAR

            IF (a = b) THEN
                result1 := 1;
            ELSE
                result1 := 0;
            END_IF

            IF (a <> b) THEN
                result2 := 1;
            ELSE
                result2 := -1;
            END_IF

           END_PROGRAM
    ";
    let _: i32 = compile_and_run(function, &mut main);
    assert_eq!([1, -1], main.results);
}

#[test]
fn less_or_equal_comparison_with_arbitrary_datatypes() {
    struct Main {
        results: [i32; 3],
    }

    let mut main = Main { results: [0, 0, 0] };

    let function = "
            FUNCTION STRING_EQUAL : BOOL
                VAR_INPUT a,b : STRING; END_VAR
                STRING_EQUAL := FALSE;
            END_FUNCTION

            FUNCTION STRING_LESS : BOOL
                VAR_INPUT a,b : STRING; END_VAR
                STRING_LESS := TRUE;
            END_FUNCTION

            PROGRAM main
            VAR
                result1 : DINT;
                result2 : DINT;
                result3 : DINT;
            END_VAR
            VAR_TEMP
                a,b : STRING[1];
            END_VAR

            IF (a = b) THEN
                result1 := 1;
            ELSE
                result1 := -1;
            END_IF

            IF (a < b) THEN
                result2 := 1;
            ELSE
                result2 := -1;
            END_IF

            IF (a <= b) THEN
                result3 := 1;
            ELSE
                result3 := -1;
            END_IF

           END_PROGRAM
    ";
    let _: i32 = compile_and_run(function, &mut main);
    assert_eq!([-1, 1, 1], main.results);
}

#[test]
fn greater_or_equal_comparison_with_arbitrary_datatypes() {
    struct Main {
        results: [i32; 3],
    }

    let mut main = Main { results: [0, 0, 0] };

    let function = "
            FUNCTION STRING_EQUAL : BOOL
                VAR_INPUT a,b : STRING; END_VAR
                STRING_EQUAL := FALSE;
            END_FUNCTION

            FUNCTION STRING_GREATER : BOOL
                VAR_INPUT a,b : STRING; END_VAR
                STRING_GREATER := TRUE;
            END_FUNCTION

            PROGRAM main
            VAR
                result1 : DINT;
                result2 : DINT;
                result3 : DINT;
            END_VAR
            VAR_TEMP
                a,b : STRING[1];
            END_VAR

            IF (a = b) THEN
                result1 := 1;
            ELSE
                result1 := -1;
            END_IF

            IF (a > b) THEN
                result2 := 1;
            ELSE
                result2 := -1;
            END_IF

            IF (a >= b) THEN
                result3 := 1;
            ELSE
                result3 := -1;
            END_IF

           END_PROGRAM
    ";
    let _: i32 = compile_and_run(function, &mut main);
    assert_eq!([-1, 1, 1], main.results);
}

#[test]
fn enums_can_be_compared() {
    struct Main {
        a: bool,
        b: bool,
        c: bool,
    }

    let mut main = Main {
        a: false,
        b: false,
        c: false,
    };

    let function = "
        TYPE MyEnum : BYTE (zero, aa, bb := 7, cc); END_TYPE

        PROGRAM main 
            VAR a,b,c : BOOL; END_VAR

            VAR_TEMP
                x : MyEnum := 1;
                y : MyEnum := bb;
                z : MyEnum := cc;
            END_VAR

            IF x = aa THEN
                a := TRUE;
            END_IF

            IF y = 7 THEN
                b := TRUE;
            END_IF
            
            IF z = 8 THEN
                c := TRUE;
            END_IF
        END_PROGRAM 
    ";
    let _: i32 = compile_and_run(function, &mut main);
    assert_eq!([true, true, true], [main.a, main.b, main.c]);
}

#[test]
fn amp_as_and_correctness_test() {
    #[derive(Default)]
    #[repr(C)]
    struct Main {
        a: bool,
        b: bool,
        c: bool,
        d: bool,
        e: bool,
    }

    let mut main = Main::default();

    let function = "
        PROGRAM main
            VAR a,b,c,d,e : BOOL; END_VAR
            a := TRUE;
            b := TRUE;
            c := FALSE;
            
            IF a & b THEN
                d := TRUE;
            END_IF

            IF a & NOT c THEN
                e := TRUE;
            END_IF

        END_PROGRAM
    ";
    let _: i32 = compile_and_run(function, &mut main);
    assert_eq!([true, true], [main.d, main.e]);
}
