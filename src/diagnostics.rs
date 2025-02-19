use std::{
    error::Error,
    fmt::{self, Display},
    ops::Range,
};

use codespan_reporting::{
    diagnostic::Label,
    files::{Files, Location, SimpleFile, SimpleFiles},
    term::termcolor::{ColorChoice, StandardStream},
};
use inkwell::support::LLVMString;

use crate::ast::{DataTypeDeclaration, DiagnosticInfo, PouType, SourceRange};

pub const INTERNAL_LLVM_ERROR: &str = "internal llvm codegen error";

#[derive(PartialEq, Debug, Clone)]
pub enum Diagnostic {
    SyntaxError {
        message: String,
        range: SourceRange,
        err_no: ErrNo,
    },
    GeneralError {
        message: String,
        err_no: ErrNo,
    },
    ImprovementSuggestion {
        message: String,
        range: SourceRange,
    },
}

#[allow(non_camel_case_types)]
#[derive(PartialEq, Debug, Clone)]
pub enum ErrNo {
    undefined,

    //general
    general__io_err,
    general__param_err,

    //syntax
    syntax__generic_error,
    syntax__missing_token,
    syntax__unexpected_token,

    //semantic
    // pou related
    pou__missing_return_type,
    pou__unexpected_return_type,
    pou__unsupported_return_type,
    pou__empty_variable_block,
    pou__missing_action_container,

    //variable related
    var__unresolved_constant,
    var__invalid_constant_block,
    var__invalid_constant,
    var__cannot_assign_to_const,
    var__invalid_assignment,
    var__missing_type,

    //reference related
    reference__unresolved,
    reference__illegal_access,

    //type related
    type__cast_error,
    type__unknown_type,
    type__literal_out_of_range,
    type__incompatible_literal_cast,
    type__incompatible_directaccess,
    type__incompatible_directaccess_variable,
    type__incompatible_directaccess_range,
    type__incompatible_arrayaccess_range,
    type__incompatible_arrayaccess_variable,
    type__incompatible_arrayaccess_type,
    type__expected_literal,
    type__invalid_nature,
    type__unknown_nature,
    type__unresolved_generic,
    type__incompatible_size,

    //codegen related
    codegen__general,
    codegen__missing_function,
    codegen__missing_compare_function,

    //linker
    linker__generic_error,
}

impl<T: Error> From<T> for Diagnostic {
    fn from(e: T) -> Self {
        Diagnostic::GeneralError {
            message: e.to_string(),
            err_no: ErrNo::general__io_err,
        }
    }
}

impl Diagnostic {
    pub fn syntax_error(message: &str, range: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: message.to_string(),
            range,
            err_no: ErrNo::syntax__generic_error,
        }
    }

    pub fn unexpected_token_found(expected: &str, found: &str, range: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "Unexpected token: expected {} but found {}",
                expected, found
            ),
            range,
            err_no: ErrNo::syntax__unexpected_token,
        }
    }

    pub fn unexpected_initializer_on_function_return(range: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: "Return types cannot have a default value".into(),
            range,
            err_no: ErrNo::syntax__unexpected_token,
        }
    }

    pub fn return_type_not_supported(pou_type: &PouType, range: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "POU Type {:?} does not support a return type. Did you mean Function?",
                pou_type
            ),
            range,
            err_no: ErrNo::pou__unexpected_return_type,
        }
    }

    pub fn function_unsupported_return_type(data_type: &DataTypeDeclaration) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "Data Type {:?} not supported as a function return type!",
                data_type
            ),
            range: data_type.get_location(),
            err_no: ErrNo::pou__unsupported_return_type,
        }
    }

    pub fn function_return_missing(range: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: "Function Return type missing".into(),
            range,
            err_no: ErrNo::pou__missing_return_type,
        }
    }

    pub fn missing_function(location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: "Cannot generate code outside of function context.".into(),
            range: location,
            err_no: ErrNo::codegen__missing_function,
        }
    }

    pub fn missing_compare_function(
        function_name: &str,
        data_type: &str,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "Missing compare function 'FUNCTION {} : BOOL VAR_INPUT a,b : {}; END_VAR ...'.",
                function_name, data_type
            ),
            range: location,
            err_no: ErrNo::codegen__missing_compare_function,
        }
    }

    pub fn missing_token(epxected_token: &str, range: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!("Missing expected Token {}", epxected_token),
            range,
            err_no: ErrNo::syntax__missing_token,
        }
    }

    pub fn missing_action_container(range: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: "Missing Actions Container Name".to_string(),
            range,
            err_no: ErrNo::pou__missing_action_container,
        }
    }

    pub fn unresolved_reference(reference: &str, location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!("Could not resolve reference to {:}", reference),
            range: location,
            err_no: ErrNo::reference__unresolved,
        }
    }

    pub fn illegal_access(reference: &str, location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!("Illegal access to private member {:}", reference),
            range: location,
            err_no: ErrNo::reference__illegal_access,
        }
    }

    pub fn unresolved_generic_type(
        symbol: &str,
        nature: &str,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "Could not resolve generic type {} with nature {}",
                symbol, nature
            ),
            range: location,
            err_no: ErrNo::type__unresolved_generic,
        }
    }

    pub fn unknown_type(type_name: &str, location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!("Unknown type: {:}", type_name),
            range: location,
            err_no: ErrNo::type__unknown_type,
        }
    }

    pub fn casting_error(type_name: &str, target_type: &str, location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!("Cannot cast from {:} to {:}", type_name, target_type),
            range: location,
            err_no: ErrNo::type__cast_error,
        }
    }

    pub fn incompatible_directaccess(
        access_type: &str,
        access_size: u64,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "{}-Wise access requires a Numerical type larger than {} bits",
                access_type, access_size
            ),
            range: location,
            err_no: ErrNo::type__incompatible_directaccess,
        }
    }

    pub fn incompatible_directaccess_range(
        access_type: &str,
        target_type: &str,
        access_range: Range<u64>,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "{}-Wise access for type {} must be in the range {}..{}",
                access_type, target_type, access_range.start, access_range.end
            ),
            range: location,
            err_no: ErrNo::type__incompatible_directaccess_range,
        }
    }

    pub fn incompatible_directaccess_variable(
        access_type: &str,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "Invalid type {} for direct variable access. Only variables of Integer types are allowed",
                access_type
            ),
            range: location,
            err_no: ErrNo::type__incompatible_directaccess_variable,
        }
    }

    pub fn incompatible_array_access_range(
        range: Range<i128>,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "Array access must be in the range {}..{}",
                range.start, range.end
            ),
            range: location,
            err_no: ErrNo::type__incompatible_arrayaccess_range,
        }
    }

    pub fn incompatible_array_access_variable(
        access_type: &str,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "Invalid type {} for array access. Only variables of Array types are allowed",
                access_type
            ),
            range: location,
            err_no: ErrNo::type__incompatible_arrayaccess_variable,
        }
    }

    pub fn incompatible_array_access_type(access_type: &str, location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "Invalid type {} for array access. Only variables of Integer types are allowed to access an array",
                access_type
            ),
            range: location,
            err_no: ErrNo::type__incompatible_arrayaccess_variable,
        }
    }

    pub fn incompatible_literal_cast(
        cast_type: &str,
        literal_type: &str,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "Literal {:} is not campatible to {:}",
                literal_type, cast_type
            ),
            range: location,
            err_no: ErrNo::type__incompatible_literal_cast,
        }
    }

    pub fn literal_expected(location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: "Expected literal".into(),
            range: location,
            err_no: ErrNo::type__expected_literal,
        }
    }

    pub fn literal_out_of_range(
        literal: &str,
        range_hint: &str,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!("Literal {:} out of range ({})", literal, range_hint),
            range: location,
            err_no: ErrNo::type__literal_out_of_range,
        }
    }

    pub fn empty_variable_block(location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: "Variable block is empty".into(),
            range: location,
            err_no: ErrNo::pou__empty_variable_block,
        }
    }

    pub fn unresolved_constant(
        constant_name: &str,
        reason: Option<&str>,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "Unresolved constant '{:}' variable{:}",
                constant_name,
                reason
                    .map(|it| format!(": {:}", it))
                    .unwrap_or_else(|| "".into()),
            ),
            range: location,
            err_no: ErrNo::pou__empty_variable_block,
        }
    }

    pub fn invalid_constant_block(location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: "This variable block does not support the CONSTANT modifier".to_string(),
            range: location,
            err_no: ErrNo::var__invalid_constant_block,
        }
    }

    pub fn invalid_constant(constant_name: &str, location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!("Invalid constant {:} - Functionblock- and Class-instances cannot be delcared constant", constant_name),
            range: location,
            err_no: ErrNo::var__invalid_constant,
        }
    }

    pub fn cannot_assign_to_constant(qualified_name: &str, location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!("Cannot assign to CONSTANT '{:}'", qualified_name),
            range: location,
            err_no: ErrNo::var__cannot_assign_to_const,
        }
    }

    pub fn cannot_generate_initializer(variable_name: &str, location: SourceRange) -> Diagnostic {
        Self::codegen_error(
            &format!(
                "Cannot generate literal initializer for '{:}': Value can not be derived",
                variable_name
            ),
            location,
        )
    }

    pub fn codegen_error(message: &str, location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: message.into(),
            range: location,
            err_no: ErrNo::codegen__general,
        }
    }

    pub fn cannot_generate_call_statement<T: DiagnosticInfo>(operator: &T) -> Diagnostic {
        Diagnostic::codegen_error(
            &format!(
                "cannot generate call statement for {:?}",
                operator.get_description()
            ),
            operator.get_location(),
        )
    }

    pub fn io_read_error(file: &str, reason: &str) -> Diagnostic {
        Diagnostic::GeneralError {
            message: format!("Cannot read file '{:}': {:}'", file, reason),
            err_no: ErrNo::general__io_err,
        }
    }

    pub fn io_write_error(file: &str, reason: &str) -> Diagnostic {
        Diagnostic::GeneralError {
            message: format!("Cannot write file '{:}': {:}'", file, reason),
            err_no: ErrNo::general__io_err,
        }
    }

    pub fn param_error(reason: &str) -> Diagnostic {
        Diagnostic::GeneralError {
            message: reason.to_string(),
            err_no: ErrNo::general__param_err,
        }
    }

    pub fn llvm_error(file: &str, llvm_error: &LLVMString) -> Diagnostic {
        Diagnostic::GeneralError {
            message: format!(
                "{:}: Internal llvm error: {:}",
                file,
                llvm_error.to_string()
            ),
            err_no: ErrNo::general__io_err,
        }
    }

    pub fn cannot_generate_from_empty_literal(
        type_name: &str,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::codegen_error(
            format!("Cannot generate {} from empty literal", type_name).as_str(),
            location,
        )
    }

    pub fn cannot_generate_string_literal(type_name: &str, location: SourceRange) -> Diagnostic {
        Diagnostic::codegen_error(
            format!("Cannot generate String-Literal for type {}", type_name).as_str(),
            location,
        )
    }

    pub fn invalid_assignment(
        right_type: &str,
        left_type: &str,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "Invalid assignment: cannot assign '{:}' to '{:}'",
                right_type, left_type
            ),
            range: location,
            err_no: ErrNo::var__invalid_assignment,
        }
    }

    pub fn invalid_type_nature(type_name: &str, nature: &str, location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "Invalid type nature for generic argument. {} is no {}.",
                type_name, nature
            ),
            range: location,
            err_no: ErrNo::type__invalid_nature,
        }
    }

    pub fn unknown_type_nature(nature: &str, location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!("Unknown type nature {}.", nature),
            range: location,
            err_no: ErrNo::type__unknown_nature,
        }
    }

    pub fn missing_datatype(reason: Option<&str>, location: SourceRange) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!("Missing datatype {}", reason.unwrap_or("")),
            range: location,
            err_no: ErrNo::var__missing_type,
        }
    }

    pub fn incompatible_type_size(
        nature: &str,
        size: u32,
        error: &str,
        location: SourceRange,
    ) -> Diagnostic {
        Diagnostic::SyntaxError {
            message: format!(
                "The type {} {} is too small to {} Pointer",
                nature, size, error
            ),
            range: location,
            err_no: ErrNo::type__incompatible_size,
        }
    }

    pub fn link_error(error: &str) -> Diagnostic {
        Diagnostic::GeneralError {
            err_no: ErrNo::linker__generic_error,
            message: error.to_string(),
        }
    }

    pub fn get_message(&self) -> &str {
        match self {
            Diagnostic::SyntaxError { message, .. }
            | Diagnostic::ImprovementSuggestion { message, .. }
            | Diagnostic::GeneralError { message, .. } => message.as_str(),
        }
    }

    pub fn get_location(&self) -> SourceRange {
        match self {
            Diagnostic::SyntaxError { range, .. }
            | Diagnostic::ImprovementSuggestion { range, .. } => range.clone(),
            Diagnostic::GeneralError { .. } => SourceRange::undefined(),
        }
    }

    pub fn get_type(&self) -> &ErrNo {
        match self {
            Diagnostic::SyntaxError { err_no, .. } | Diagnostic::GeneralError { err_no, .. } => {
                err_no
            }
            Diagnostic::ImprovementSuggestion { .. } => &ErrNo::undefined,
        }
    }

    /**
     * relocates the given diagnostic to the given location if possible and returns it back
     */
    pub fn relocate(it: Diagnostic, new_location: SourceRange) -> Diagnostic {
        match it {
            Diagnostic::SyntaxError {
                message, err_no, ..
            } => Diagnostic::SyntaxError {
                message,
                range: new_location,
                err_no,
            },
            Diagnostic::ImprovementSuggestion { message, .. } => {
                Diagnostic::ImprovementSuggestion {
                    message,
                    range: new_location,
                }
            }
            _ => it,
        }
    }

    pub fn invalid_pragma_location(message: &str, range: SourceRange) -> Diagnostic {
        Diagnostic::ImprovementSuggestion {
            message: format!("Invalid pragma location: {}", message),
            range,
        }
    }
}

/// a diagnostics severity
pub enum Severity {
    Error,
    Warning,
    _Info,
}

impl Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::_Info => "info",
        };
        write!(f, "{}", severity)
    }
}

/// the assessor determins the severity of a diagnostic
/// this trait allows for different implementations for different usecases
/// (e.g. default, compiler-settings, tests)
pub trait DiagnosticAssessor {
    /// determines the severity of the given diagnostic
    fn assess(&self, d: Diagnostic) -> AssessedDiagnostic;
    fn assess_all(&self, d: Vec<Diagnostic>) -> Vec<AssessedDiagnostic> {
        d.into_iter().map(|it| self.assess(it)).collect()
    }
}

/// the default assessor will treat ImprovementSuggestions as warnings
/// and everything else as errors
#[derive(Default)]
pub struct DefaultDiagnosticAssessor {}

pub struct AssessedDiagnostic {
    pub diagnostic: Diagnostic,
    pub severity: Severity,
}

impl DiagnosticAssessor for DefaultDiagnosticAssessor {
    fn assess(&self, d: Diagnostic) -> AssessedDiagnostic {
        let severity = match d {
            // improvements become warnings
            Diagnostic::ImprovementSuggestion { .. } => Severity::Warning,
            // everything else becomes an error
            _ => Severity::Error,
        };

        AssessedDiagnostic {
            diagnostic: d,
            severity,
        }
    }
}

/// the DiagnosticReporter decides on the format and where to report the diagnostic to.
/// possible implementations could print to either std-out, std-err or a file, etc.
pub trait DiagnosticReporter {
    /// reports the given diagnostic
    fn report(&self, diagnostics: &[AssessedDiagnostic], file_id: usize);
    /// register the given path & src and returns an ID to indicate
    /// a relationship the given src (diagnostics for this src need
    /// to use this id)
    fn register(&mut self, path: String, src: String) -> usize;
}

/// a DiagnosticReporter that reports diagnostics using codespan_reporting
pub struct CodeSpanDiagnosticReporter {
    files: SimpleFiles<String, String>,
    config: codespan_reporting::term::Config,
    writer: StandardStream,
}

impl CodeSpanDiagnosticReporter {
    /// creates a new reporter with the given codespan_reporting configuration
    fn new(config: codespan_reporting::term::Config, writer: StandardStream) -> Self {
        CodeSpanDiagnosticReporter {
            files: SimpleFiles::new(),
            config,
            writer,
        }
    }
}

impl Default for CodeSpanDiagnosticReporter {
    /// creates the default CodeSpanDiagnosticReporter reporting to StdErr, with colors
    fn default() -> Self {
        Self::new(
            codespan_reporting::term::Config {
                display_style: codespan_reporting::term::DisplayStyle::Rich,
                tab_width: 2,
                styles: codespan_reporting::term::Styles::default(),
                chars: codespan_reporting::term::Chars::default(),
                start_context_lines: 5,
                end_context_lines: 3,
            },
            StandardStream::stderr(ColorChoice::Always),
        )
    }
}

impl DiagnosticReporter for CodeSpanDiagnosticReporter {
    fn report(&self, diagnostics: &[AssessedDiagnostic], file_id: usize) {
        for ad in diagnostics {
            let d = &ad.diagnostic;
            let location = d.get_location();

            let diagnostic_factory = match ad.severity {
                Severity::Error => codespan_reporting::diagnostic::Diagnostic::error(),
                Severity::Warning => codespan_reporting::diagnostic::Diagnostic::warning(),
                Severity::_Info => codespan_reporting::diagnostic::Diagnostic::note(),
            };

            let diag = diagnostic_factory
                .with_message(d.get_message())
                .with_labels(vec![Label::primary(
                    file_id,
                    location.get_start()..location.get_end(),
                )]);
            let result = codespan_reporting::term::emit(
                &mut self.writer.lock(),
                &self.config,
                &self.files,
                &diag,
            );
            if let Err(err) = result {
                eprintln!("Unable to report diagnostics: {}", err);
            }
        }
    }

    fn register(&mut self, path: String, src: String) -> usize {
        self.files.add(path, src)
    }
}

/// a DiagnosticReporter that reports diagnostics using clang-format
pub struct ClangFormatDiagnosticReporter {
    files: SimpleFiles<String, String>,
}

impl ClangFormatDiagnosticReporter {
    fn new() -> Self {
        ClangFormatDiagnosticReporter {
            files: SimpleFiles::new(),
        }
    }
}

impl Default for ClangFormatDiagnosticReporter {
    fn default() -> Self {
        ClangFormatDiagnosticReporter::new()
    }
}

impl DiagnosticReporter for ClangFormatDiagnosticReporter {
    fn report(&self, diagnostics: &[AssessedDiagnostic], file_id: usize) {
        for ad in diagnostics {
            let file = self.files.get(file_id).ok();

            let diagnostic = &ad.diagnostic;
            let location = &diagnostic.get_location();
            let start = self.files.location(file_id, location.get_start());
            let end = self.files.location(file_id, location.get_end());

            let res = self.build_diagnostic_msg(
                file,
                start.as_ref().ok(),
                end.as_ref().ok(),
                &ad.severity,
                diagnostic.get_message(),
            );

            eprintln!("{}", res);
        }
    }
    fn register(&mut self, path: String, src: String) -> usize {
        self.files.add(path, src)
    }
}

impl ClangFormatDiagnosticReporter {
    /// returns diagnostic message in clang format
    /// file-name:{range}: severity: message
    /// optional parameters that are none will not be included
    fn build_diagnostic_msg(
        &self,
        file: Option<&SimpleFile<String, String>>,
        start: Option<&Location>,
        end: Option<&Location>,
        severity: &Severity,
        msg: &str,
    ) -> String {
        let mut str = String::new();
        // file name
        if let Some(f) = file {
            str.push_str(format!("{}:", f.name().as_str()).as_str());
            // range
            if let Some(s) = start {
                if let Some(e) = end {
                    // if start and end are equal there is no need to show the range
                    if s.eq(e) {
                        str.push_str(format!("{}:{}: ", s.line_number, s.column_number).as_str());
                    } else {
                        str.push_str(
                            format!(
                                "{}:{}:{{{}:{}-{}:{}}}: ",
                                s.line_number,
                                s.column_number,
                                s.line_number,
                                s.column_number,
                                e.line_number,
                                e.column_number
                            )
                            .as_str(),
                        );
                    }
                }
            } else {
                str.push(' ');
            }
        }
        // severity
        str.push_str(format!("{}: ", severity).as_str());
        // msg
        str.push_str(msg);

        str
    }
}

/// a DiagnosticReporter that swallows all diagnostics
#[derive(Default)]
pub struct NullDiagnosticReporter {
    last_id: usize,
}

impl DiagnosticReporter for NullDiagnosticReporter {
    fn report(&self, _diagnostics: &[AssessedDiagnostic], _file_id: usize) {
        //ignore
    }

    fn register(&mut self, _path: String, _src: String) -> usize {
        // at least provide some unique ids
        self.last_id += 1;
        self.last_id
    }
}

/// the Diagnostician handle's Diangostics with the help of a
/// assessor and a reporter
pub struct Diagnostician {
    pub reporter: Box<dyn DiagnosticReporter>,
    pub assessor: Box<dyn DiagnosticAssessor>,
}

impl Diagnostician {
    /// registers the given source-code at the diagnostician, so it can
    /// preview errors in the source
    /// returns the id to use to reference the given file
    pub fn register_file(&mut self, id: String, src: String) -> usize {
        self.reporter.register(id, src)
    }

    /// creates a null-diagnostician that does not report diagnostics
    pub fn null_diagnostician() -> Diagnostician {
        Diagnostician {
            assessor: Box::new(DefaultDiagnosticAssessor::default()),
            reporter: Box::new(NullDiagnosticReporter::default()),
        }
    }

    /// creates a clang-format-diagnostician that reports diagnostics in clang format
    pub fn clang_format_diagnostician() -> Diagnostician {
        Diagnostician {
            reporter: Box::new(ClangFormatDiagnosticReporter::default()),
            assessor: Box::new(DefaultDiagnosticAssessor::default()),
        }
    }

    /// assess and reports the given diagnostics
    pub fn handle(&self, diagnostics: Vec<Diagnostic>, file_id: usize) {
        self.report(&self.assess_all(diagnostics), file_id);
    }
}

impl DiagnosticReporter for Diagnostician {
    fn report(&self, diagnostics: &[AssessedDiagnostic], file_id: usize) {
        //delegate to reporter
        self.reporter.report(diagnostics, file_id);
    }

    fn register(&mut self, path: String, src: String) -> usize {
        //delegate to reporter
        self.reporter.register(path, src)
    }
}

impl DiagnosticAssessor for Diagnostician {
    fn assess(&self, d: Diagnostic) -> AssessedDiagnostic {
        //delegate to assesor
        self.assessor.assess(d)
    }
}

impl Default for Diagnostician {
    fn default() -> Self {
        Self {
            reporter: Box::new(CodeSpanDiagnosticReporter::default()),
            assessor: Box::new(DefaultDiagnosticAssessor::default()),
        }
    }
}

#[cfg(test)]
mod diagnostics_tests {
    use codespan_reporting::files::{Location, SimpleFile};

    use super::ClangFormatDiagnosticReporter;

    #[test]
    fn test_build_diagnostic_msg() {
        let reporter = ClangFormatDiagnosticReporter::default();
        let file = SimpleFile::new("test.st".to_string(), "source".to_string());
        let start = Location {
            line_number: 4,
            column_number: 1,
        };
        let end = Location {
            line_number: 4,
            column_number: 4,
        };
        let res = reporter.build_diagnostic_msg(
            Some(&file),
            Some(&start),
            Some(&end),
            &super::Severity::Error,
            "This is an error",
        );

        assert_eq!(res, "test.st:4:1:{4:1-4:4}: error: This is an error");
    }

    #[test]
    fn test_build_diagnostic_msg_equal_start_end() {
        let reporter = ClangFormatDiagnosticReporter::default();
        let file = SimpleFile::new("test.st".to_string(), "source".to_string());
        let start = Location {
            line_number: 4,
            column_number: 1,
        };
        let end = Location {
            line_number: 4,
            column_number: 1,
        };
        let res = reporter.build_diagnostic_msg(
            Some(&file),
            Some(&start),
            Some(&end),
            &super::Severity::Error,
            "This is an error",
        );

        assert_eq!(res, "test.st:4:1: error: This is an error");
    }

    #[test]
    fn test_build_diagnostic_msg_no_location() {
        let reporter = ClangFormatDiagnosticReporter::default();
        let file = SimpleFile::new("test.st".to_string(), "source".to_string());
        let res = reporter.build_diagnostic_msg(
            Some(&file),
            None,
            None,
            &super::Severity::Error,
            "This is an error",
        );

        assert_eq!(res, "test.st: error: This is an error");
    }

    #[test]
    fn test_build_diagnostic_msg_no_file() {
        let reporter = ClangFormatDiagnosticReporter::default();
        let start = Location {
            line_number: 4,
            column_number: 1,
        };
        let end = Location {
            line_number: 4,
            column_number: 4,
        };
        let res = reporter.build_diagnostic_msg(
            None,
            Some(&start),
            Some(&end),
            &super::Severity::Error,
            "This is an error",
        );

        assert_eq!(res, "error: This is an error");
    }

    #[test]
    fn test_build_diagnostic_msg_no_file_no_location() {
        let reporter = ClangFormatDiagnosticReporter::default();
        let res = reporter.build_diagnostic_msg(
            None,
            None,
            None,
            &super::Severity::Error,
            "This is an error",
        );

        assert_eq!(res, "error: This is an error");
    }
}
