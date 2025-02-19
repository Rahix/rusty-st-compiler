// Copyright (c) 2020 Ghaith Hachem and Mathias Rieder
use crate::{
    lexer::IdProvider,
    typesystem::{DataTypeInformation, REAL_TYPE, VOID_TYPE},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display, Formatter, Result},
    iter,
    ops::Range,
    unimplemented,
};
mod pre_processor;

pub type AstId = usize;

#[derive(Clone, Debug, PartialEq)]
pub struct GenericBinding {
    pub name: String,
    pub nature: TypeNature,
}

#[derive(PartialEq)]
pub struct Pou {
    pub name: String,
    pub variable_blocks: Vec<VariableBlock>,
    pub pou_type: PouType,
    pub return_type: Option<DataTypeDeclaration>,
    /// the SourceRange of the whole POU
    pub location: SourceRange,
    /// the SourceRange of the POU's name
    pub name_location: SourceRange,
    pub poly_mode: Option<PolymorphismMode>,
    pub generics: Vec<GenericBinding>,
    pub linkage: LinkageType,
}

#[derive(Debug, PartialEq)]
pub enum PolymorphismMode {
    None,
    Abstract,
    Final,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "direction")]
pub enum HardwareAccessType {
    Input,
    Output,
    Memory,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum DirectAccessType {
    Bit,
    Byte,
    Word,
    DWord,
    Template,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TypeNature {
    Any,
    Derived,
    Elementary,
    Magnitude,
    Num,
    Real,
    Int,
    Signed,
    Unsigned,
    Duration,
    Bit,
    Chars,
    String,
    Char,
    Date,
}

impl TypeNature {
    pub fn get_smallest_possible_type(&self) -> &str {
        match self {
            TypeNature::Real => REAL_TYPE,
            _ => "",
        }
    }

    pub fn derives(self, other: TypeNature) -> bool {
        if other == self {
            true
        } else {
            match self {
                TypeNature::Any => true,
                TypeNature::Derived => matches!(other, TypeNature::Any),
                TypeNature::Elementary => matches!(other, TypeNature::Any),
                TypeNature::Magnitude => matches!(other, TypeNature::Elementary | TypeNature::Any),
                TypeNature::Num => matches!(
                    other,
                    TypeNature::Magnitude | TypeNature::Elementary | TypeNature::Any
                ),
                TypeNature::Real => matches!(
                    other,
                    TypeNature::Num
                        | TypeNature::Magnitude
                        | TypeNature::Elementary
                        | TypeNature::Any
                ),
                TypeNature::Int => matches!(
                    other,
                    TypeNature::Num
                        | TypeNature::Magnitude
                        | TypeNature::Elementary
                        | TypeNature::Any
                ),
                TypeNature::Signed => matches!(
                    other,
                    TypeNature::Int
                        | TypeNature::Num
                        | TypeNature::Magnitude
                        | TypeNature::Elementary
                        | TypeNature::Any
                ),
                TypeNature::Unsigned => matches!(
                    other,
                    TypeNature::Int
                        | TypeNature::Num
                        | TypeNature::Magnitude
                        | TypeNature::Elementary
                        | TypeNature::Any
                ),
                TypeNature::Duration => matches!(
                    other,
                    TypeNature::Magnitude | TypeNature::Elementary | TypeNature::Any
                ),
                TypeNature::Bit => matches!(other, TypeNature::Elementary | TypeNature::Any),
                TypeNature::Chars => matches!(other, TypeNature::Elementary | TypeNature::Any),
                TypeNature::String => matches!(
                    other,
                    TypeNature::Chars | TypeNature::Elementary | TypeNature::Any
                ),
                TypeNature::Char => matches!(
                    other,
                    TypeNature::Chars | TypeNature::Elementary | TypeNature::Any
                ),
                TypeNature::Date => matches!(other, TypeNature::Elementary | TypeNature::Any),
            }
        }
    }
}

impl DirectAccessType {
    /// Returns true if the current index is in the range for the given type
    pub fn is_in_range(&self, index: u64, data_type: &DataTypeInformation) -> bool {
        (self.get_bit_width() * index) < data_type.get_size() as u64
    }

    /// Returns the range from 0 for the given data type
    pub fn get_range(&self, data_type: &DataTypeInformation) -> Range<u64> {
        0..((data_type.get_size() as u64 / self.get_bit_width()) - 1)
    }

    /// Returns true if the direct access can be used for the given type
    pub fn is_compatible(&self, data_type: &DataTypeInformation) -> bool {
        data_type.get_semantic_size() as u64 > self.get_bit_width()
    }

    /// Returns the size of the bitaccess result
    pub fn get_bit_width(&self) -> u64 {
        match self {
            DirectAccessType::Bit => 1,
            DirectAccessType::Byte => 8,
            DirectAccessType::Word => 16,
            DirectAccessType::DWord => 32,
            DirectAccessType::Template => unimplemented!("Should not test for template width"),
        }
    }
}

impl Debug for Pou {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut str = f.debug_struct("POU");
        str.field("name", &self.name)
            .field("variable_blocks", &self.variable_blocks)
            .field("pou_type", &self.pou_type)
            .field("return_type", &self.return_type);
        if !self.generics.is_empty() {
            str.field("generics", &self.generics);
        }
        str.finish()
    }
}

impl Pou {
    pub fn get_return_name(&self) -> &str {
        Pou::calc_return_name(&self.name)
    }

    pub fn calc_return_name(pou_name: &str) -> &str {
        pou_name.split('.').last().unwrap_or_default()
    }
}

#[derive(Debug, PartialEq)]
pub struct Implementation {
    pub name: String,
    pub type_name: String,
    pub linkage: LinkageType,
    pub pou_type: PouType,
    pub statements: Vec<AstStatement>,
    pub location: SourceRange,
    pub overriding: bool,
    pub generic: bool,
    pub access: Option<AccessModifier>,
}

#[derive(Debug, Copy, PartialEq, Clone)]
pub enum LinkageType {
    Internal,
    External,
    BuiltIn,
}

#[derive(Debug, PartialEq)]
pub enum AccessModifier {
    Private,
    Public,
    Protected, // default
    Internal,
}

#[derive(Debug, PartialEq, Clone)]
pub enum PouType {
    Program,
    Function,
    FunctionBlock,
    Action,
    Class,
    Method { owner_class: String },
}

impl PouType {
    /// returns Some(owner_class) if this is a `Method` or otherwhise `None`
    pub fn get_optional_owner_class(&self) -> Option<String> {
        if let PouType::Method { owner_class } = self {
            Some(owner_class.clone())
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct CompilationUnit {
    pub global_vars: Vec<VariableBlock>,
    pub units: Vec<Pou>,
    pub implementations: Vec<Implementation>,
    pub types: Vec<UserTypeDeclaration>,
}

impl CompilationUnit {
    /// imports all elements of the other CompilationUnit into this CompilationUnit
    ///
    /// this will import all global_vars, units, implementations and types. The imported
    /// structs are moved from the other unit into this unit
    /// # Arguments
    /// `other` the other CompilationUnit to import the elements from.
    pub fn import(&mut self, other: CompilationUnit) {
        self.global_vars.extend(other.global_vars);
        self.units.extend(other.units);
        self.implementations.extend(other.implementations);
        self.types.extend(other.types);
    }
}

#[derive(Debug, Copy, PartialEq, Clone)]
pub enum VariableBlockType {
    Local,
    Temp,
    Input(ArgumentProperty),
    Output,
    Global,
    InOut,
}

#[derive(Debug, Copy, PartialEq, Clone)]
pub enum ArgumentProperty {
    ByVal,
    ByRef,
}

#[derive(PartialEq)]
pub struct VariableBlock {
    pub access: AccessModifier,
    pub constant: bool,
    pub retain: bool,
    pub variables: Vec<Variable>,
    pub variable_block_type: VariableBlockType,
    pub linkage: LinkageType,
    pub location: SourceRange,
}

impl Debug for VariableBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("VariableBlock")
            .field("variables", &self.variables)
            .field("variable_block_type", &self.variable_block_type)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub struct Variable {
    pub name: String,
    pub data_type: DataTypeDeclaration,
    pub initializer: Option<AstStatement>,
    pub address: Option<AstStatement>,
    pub location: SourceRange,
}

impl Debug for Variable {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut var = f.debug_struct("Variable");
        var.field("name", &self.name)
            .field("data_type", &self.data_type);
        if self.initializer.is_some() {
            var.field("initializer", &self.initializer);
        }
        if self.address.is_some() {
            var.field("address", &self.address);
        }
        var.finish()
    }
}

impl Variable {
    pub fn replace_data_type_with_reference_to(
        &mut self,
        type_name: String,
    ) -> DataTypeDeclaration {
        let new_data_type = DataTypeDeclaration::DataTypeReference {
            referenced_type: type_name,
            location: self.data_type.get_location(),
        };
        std::mem::replace(&mut self.data_type, new_data_type)
    }
}

pub trait DiagnosticInfo {
    fn get_description(&self) -> String;
    fn get_location(&self) -> SourceRange;
}

impl DiagnosticInfo for AstStatement {
    fn get_description(&self) -> String {
        format!("{:?}", self)
    }

    fn get_location(&self) -> SourceRange {
        self.get_location()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceRange {
    range: core::ops::Range<usize>,
}

impl SourceRange {
    pub fn new(range: core::ops::Range<usize>) -> SourceRange {
        SourceRange { range }
    }

    pub fn undefined() -> SourceRange {
        SourceRange { range: 0..0 }
    }

    pub fn get_start(&self) -> usize {
        self.range.start
    }

    pub fn get_end(&self) -> usize {
        self.range.end
    }

    pub fn sub_range(&self, start: usize, len: usize) -> SourceRange {
        SourceRange::new((self.get_start() + start)..(self.get_start() + len))
    }

    pub fn to_range(&self) -> Range<usize> {
        self.range.clone()
    }
}

impl From<std::ops::Range<usize>> for SourceRange {
    fn from(range: std::ops::Range<usize>) -> SourceRange {
        SourceRange::new(range)
    }
}

#[derive(Clone, PartialEq)]
pub enum DataTypeDeclaration {
    DataTypeReference {
        referenced_type: String,
        location: SourceRange,
    },
    DataTypeDefinition {
        data_type: DataType,
        location: SourceRange,
        scope: Option<String>,
    },
}

impl Debug for DataTypeDeclaration {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            DataTypeDeclaration::DataTypeReference {
                referenced_type, ..
            } => f
                .debug_struct("DataTypeReference")
                .field("referenced_type", referenced_type)
                .finish(),
            DataTypeDeclaration::DataTypeDefinition { data_type, .. } => f
                .debug_struct("DataTypeDefinition")
                .field("data_type", data_type)
                .finish(),
        }
    }
}

impl DataTypeDeclaration {
    pub fn get_name(&self) -> Option<&str> {
        match self {
            DataTypeDeclaration::DataTypeReference {
                referenced_type, ..
            } => Some(referenced_type.as_str()),
            DataTypeDeclaration::DataTypeDefinition { data_type, .. } => data_type.get_name(),
        }
    }

    pub fn get_location(&self) -> SourceRange {
        match self {
            DataTypeDeclaration::DataTypeReference { location, .. } => location.clone(),
            DataTypeDeclaration::DataTypeDefinition { location, .. } => location.clone(),
        }
    }
}

#[derive(PartialEq)]
pub struct UserTypeDeclaration {
    pub data_type: DataType,
    pub initializer: Option<AstStatement>,
    pub location: SourceRange,
    /// stores the original scope for compiler-generated types
    pub scope: Option<String>,
}

impl Debug for UserTypeDeclaration {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("UserTypeDeclaration")
            .field("data_type", &self.data_type)
            .field("initializer", &self.initializer)
            .field("scope", &self.scope)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    StructType {
        name: Option<String>, //maybe None for inline structs
        variables: Vec<Variable>,
    },
    EnumType {
        name: Option<String>, //maybe empty for inline enums
        numeric_type: String,
        elements: AstStatement, //a single Ref, or an ExpressionList with Refs
    },
    SubRangeType {
        name: Option<String>,
        referenced_type: String,
        bounds: Option<AstStatement>,
    },
    ArrayType {
        name: Option<String>,
        bounds: AstStatement,
        referenced_type: Box<DataTypeDeclaration>,
    },
    PointerType {
        name: Option<String>,
        referenced_type: Box<DataTypeDeclaration>,
    },
    StringType {
        name: Option<String>,
        is_wide: bool, //WSTRING
        size: Option<AstStatement>,
    },
    VarArgs {
        referenced_type: Option<Box<DataTypeDeclaration>>,
        sized: bool, //If the variadic has the sized property
    },
    GenericType {
        name: String,
        generic_symbol: String,
        nature: TypeNature,
    },
}

impl DataType {
    pub fn set_name(&mut self, new_name: String) {
        match self {
            DataType::StructType { name, .. }
            | DataType::EnumType { name, .. }
            | DataType::SubRangeType { name, .. }
            | DataType::ArrayType { name, .. }
            | DataType::PointerType { name, .. }
            | DataType::StringType { name, .. } => *name = Some(new_name),
            DataType::GenericType { name, .. } => *name = new_name,
            DataType::VarArgs { .. } => {} //No names on varargs
        }
    }

    pub fn get_name(&self) -> Option<&str> {
        match &self {
            DataType::StructType { name, .. }
            | DataType::EnumType { name, .. }
            | DataType::ArrayType { name, .. }
            | DataType::PointerType { name, .. }
            | DataType::StringType { name, .. }
            | DataType::SubRangeType { name, .. } => name.as_ref().map(|x| x.as_str()),
            DataType::GenericType { name, .. } => Some(name.as_str()),
            DataType::VarArgs {
                referenced_type, ..
            } => referenced_type
                .as_ref()
                .and_then(|it| DataTypeDeclaration::get_name(it.as_ref()))
                .or(Some(VOID_TYPE)),
        }
    }

    //Attempts to replace the inner type with a reference. Returns the old type if replaceable
    pub fn replace_data_type_with_reference_to(
        &mut self,
        type_name: String,
        location: &SourceRange,
    ) -> Option<DataTypeDeclaration> {
        match self {
            DataType::ArrayType {
                referenced_type, ..
            } => replace_reference(referenced_type, type_name, location),
            DataType::PointerType {
                referenced_type, ..
            } => replace_reference(referenced_type, type_name, location),
            _ => None,
        }
    }
}

fn replace_reference(
    referenced_type: &mut Box<DataTypeDeclaration>,
    type_name: String,
    location: &SourceRange,
) -> Option<DataTypeDeclaration> {
    if let DataTypeDeclaration::DataTypeReference { .. } = **referenced_type {
        return None;
    }
    let new_data_type = DataTypeDeclaration::DataTypeReference {
        referenced_type: type_name,
        location: location.clone(),
    };
    let old_data_type = std::mem::replace(referenced_type, Box::new(new_data_type));
    Some(*old_data_type)
}

#[derive(Clone, PartialEq)]
pub struct ConditionalBlock {
    pub condition: Box<AstStatement>,
    pub body: Vec<AstStatement>,
}

impl Debug for ConditionalBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ConditionalBlock")
            .field("condition", &self.condition)
            .field("body", &self.body)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub enum AstStatement {
    EmptyStatement {
        location: SourceRange,
        id: AstId,
    },
    // Literals
    LiteralInteger {
        value: i128,
        location: SourceRange,
        id: AstId,
    },
    LiteralDate {
        year: i32,
        month: u32,
        day: u32,
        location: SourceRange,
        id: AstId,
    },
    LiteralDateAndTime {
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        min: u32,
        sec: u32,
        milli: u32,
        location: SourceRange,
        id: AstId,
    },
    LiteralTimeOfDay {
        hour: u32,
        min: u32,
        sec: u32,
        milli: u32,
        location: SourceRange,
        id: AstId,
    },
    LiteralTime {
        day: f64,
        hour: f64,
        min: f64,
        sec: f64,
        milli: f64,
        micro: f64,
        nano: u32,
        negative: bool,
        location: SourceRange,
        id: AstId,
    },
    LiteralReal {
        value: String,
        location: SourceRange,
        id: AstId,
    },
    LiteralBool {
        value: bool,
        location: SourceRange,
        id: AstId,
    },
    LiteralString {
        value: String,
        is_wide: bool,
        location: SourceRange,
        id: AstId,
    },
    LiteralArray {
        elements: Option<Box<AstStatement>>, // expression-list
        location: SourceRange,
        id: AstId,
    },
    CastStatement {
        target: Box<AstStatement>,
        type_name: String,
        location: SourceRange,
        id: AstId,
    },
    MultipliedStatement {
        multiplier: u32,
        element: Box<AstStatement>,
        location: SourceRange,
        id: AstId,
    },
    // Expressions
    QualifiedReference {
        elements: Vec<AstStatement>,
        id: AstId,
    },
    Reference {
        name: String,
        location: SourceRange,
        id: AstId,
    },
    ArrayAccess {
        reference: Box<AstStatement>,
        access: Box<AstStatement>,
        id: AstId,
    },
    PointerAccess {
        reference: Box<AstStatement>,
        id: AstId,
    },
    DirectAccess {
        access: DirectAccessType,
        index: Box<AstStatement>,
        location: SourceRange,
        id: AstId,
    },
    HardwareAccess {
        direction: HardwareAccessType,
        access: DirectAccessType,
        address: Vec<AstStatement>,
        location: SourceRange,
        id: AstId,
    },
    BinaryExpression {
        operator: Operator,
        left: Box<AstStatement>,
        right: Box<AstStatement>,
        id: AstId,
    },
    UnaryExpression {
        operator: Operator,
        value: Box<AstStatement>,
        location: SourceRange,
        id: AstId,
    },
    ExpressionList {
        expressions: Vec<AstStatement>,
        id: AstId,
    },
    RangeStatement {
        start: Box<AstStatement>,
        end: Box<AstStatement>,
        id: AstId,
    },
    // Assignment
    Assignment {
        left: Box<AstStatement>,
        right: Box<AstStatement>,
        id: AstId,
    },
    // OutputAssignment
    OutputAssignment {
        left: Box<AstStatement>,
        right: Box<AstStatement>,
        id: AstId,
    },
    //Call Statement
    CallStatement {
        operator: Box<AstStatement>,
        parameters: Box<Option<AstStatement>>,
        location: SourceRange,
        id: AstId,
    },
    // Control Statements
    IfStatement {
        blocks: Vec<ConditionalBlock>,
        else_block: Vec<AstStatement>,
        location: SourceRange,
        id: AstId,
    },
    ForLoopStatement {
        counter: Box<AstStatement>,
        start: Box<AstStatement>,
        end: Box<AstStatement>,
        by_step: Option<Box<AstStatement>>,
        body: Vec<AstStatement>,
        location: SourceRange,
        id: AstId,
    },
    WhileLoopStatement {
        condition: Box<AstStatement>,
        body: Vec<AstStatement>,
        location: SourceRange,
        id: AstId,
    },
    RepeatLoopStatement {
        condition: Box<AstStatement>,
        body: Vec<AstStatement>,
        location: SourceRange,
        id: AstId,
    },
    CaseStatement {
        selector: Box<AstStatement>,
        case_blocks: Vec<ConditionalBlock>,
        else_block: Vec<AstStatement>,
        location: SourceRange,
        id: AstId,
    },
    CaseCondition {
        condition: Box<AstStatement>,
        id: AstId,
    },
    ExitStatement {
        location: SourceRange,
        id: AstId,
    },
    ContinueStatement {
        location: SourceRange,
        id: AstId,
    },
    ReturnStatement {
        location: SourceRange,
        id: AstId,
    },
    LiteralNull {
        location: SourceRange,
        id: AstId,
    },
}

impl Debug for AstStatement {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            AstStatement::EmptyStatement { .. } => f.debug_struct("EmptyStatement").finish(),
            AstStatement::LiteralNull { .. } => f.debug_struct("LiteralNull").finish(),
            AstStatement::LiteralInteger { value, .. } => f
                .debug_struct("LiteralInteger")
                .field("value", value)
                .finish(),
            AstStatement::LiteralDate {
                year, month, day, ..
            } => f
                .debug_struct("LiteralDate")
                .field("year", year)
                .field("month", month)
                .field("day", day)
                .finish(),
            AstStatement::LiteralDateAndTime {
                year,
                month,
                day,
                hour,
                min,
                sec,
                milli,
                ..
            } => f
                .debug_struct("LiteralDateAndTime")
                .field("year", year)
                .field("month", month)
                .field("day", day)
                .field("hour", hour)
                .field("min", min)
                .field("sec", sec)
                .field("milli", milli)
                .finish(),
            AstStatement::LiteralTimeOfDay {
                hour,
                min,
                sec,
                milli,
                ..
            } => f
                .debug_struct("LiteralTimeOfDay")
                .field("hour", hour)
                .field("min", min)
                .field("sec", sec)
                .field("milli", milli)
                .finish(),
            AstStatement::LiteralTime {
                day,
                hour,
                min,
                sec,
                milli,
                micro,
                nano,
                negative,
                ..
            } => f
                .debug_struct("LiteralTime")
                .field("day", day)
                .field("hour", hour)
                .field("min", min)
                .field("sec", sec)
                .field("milli", milli)
                .field("micro", micro)
                .field("nano", nano)
                .field("negative", negative)
                .finish(),
            AstStatement::LiteralReal { value, .. } => {
                f.debug_struct("LiteralReal").field("value", value).finish()
            }
            AstStatement::LiteralBool { value, .. } => {
                f.debug_struct("LiteralBool").field("value", value).finish()
            }
            AstStatement::LiteralString { value, is_wide, .. } => f
                .debug_struct("LiteralString")
                .field("value", value)
                .field("is_wide", is_wide)
                .finish(),
            AstStatement::LiteralArray { elements, .. } => f
                .debug_struct("LiteralArray")
                .field("elements", elements)
                .finish(),
            AstStatement::Reference { name, .. } => {
                f.debug_struct("Reference").field("name", name).finish()
            }
            AstStatement::QualifiedReference { elements, .. } => f
                .debug_struct("QualifiedReference")
                .field("elements", elements)
                .finish(),
            AstStatement::BinaryExpression {
                operator,
                left,
                right,
                ..
            } => f
                .debug_struct("BinaryExpression")
                .field("operator", operator)
                .field("left", left)
                .field("right", right)
                .finish(),
            AstStatement::UnaryExpression {
                operator, value, ..
            } => f
                .debug_struct("UnaryExpression")
                .field("operator", operator)
                .field("value", value)
                .finish(),
            AstStatement::ExpressionList { expressions, .. } => f
                .debug_struct("ExpressionList")
                .field("expressions", expressions)
                .finish(),
            AstStatement::RangeStatement { start, end, .. } => f
                .debug_struct("RangeStatement")
                .field("start", start)
                .field("end", end)
                .finish(),
            AstStatement::Assignment { left, right, .. } => f
                .debug_struct("Assignment")
                .field("left", left)
                .field("right", right)
                .finish(),
            AstStatement::OutputAssignment { left, right, .. } => f
                .debug_struct("OutputAssignment")
                .field("left", left)
                .field("right", right)
                .finish(),
            AstStatement::CallStatement {
                operator,
                parameters,
                ..
            } => f
                .debug_struct("CallStatement")
                .field("operator", operator)
                .field("parameters", parameters)
                .finish(),
            AstStatement::IfStatement {
                blocks, else_block, ..
            } => f
                .debug_struct("IfStatement")
                .field("blocks", blocks)
                .field("else_block", else_block)
                .finish(),
            AstStatement::ForLoopStatement {
                counter,
                start,
                end,
                by_step,
                body,
                ..
            } => f
                .debug_struct("ForLoopStatement")
                .field("counter", counter)
                .field("start", start)
                .field("end", end)
                .field("by_step", by_step)
                .field("body", body)
                .finish(),
            AstStatement::WhileLoopStatement {
                condition, body, ..
            } => f
                .debug_struct("WhileLoopStatement")
                .field("condition", condition)
                .field("body", body)
                .finish(),
            AstStatement::RepeatLoopStatement {
                condition, body, ..
            } => f
                .debug_struct("RepeatLoopStatement")
                .field("condition", condition)
                .field("body", body)
                .finish(),
            AstStatement::CaseStatement {
                selector,
                case_blocks,
                else_block,
                ..
            } => f
                .debug_struct("CaseStatement")
                .field("selector", selector)
                .field("case_blocks", case_blocks)
                .field("else_block", else_block)
                .finish(),
            AstStatement::ArrayAccess {
                reference, access, ..
            } => f
                .debug_struct("ArrayAccess")
                .field("reference", reference)
                .field("access", access)
                .finish(),
            AstStatement::PointerAccess { reference, .. } => f
                .debug_struct("PointerAccess")
                .field("reference", reference)
                .finish(),
            AstStatement::DirectAccess { access, index, .. } => f
                .debug_struct("DirectAccess")
                .field("access", access)
                .field("index", index)
                .finish(),
            AstStatement::HardwareAccess {
                direction,
                access,
                address,
                location,
                ..
            } => f
                .debug_struct("HardwareAccess")
                .field("direction", direction)
                .field("access", access)
                .field("address", address)
                .field("location", location)
                .finish(),
            AstStatement::MultipliedStatement {
                multiplier,
                element,
                ..
            } => f
                .debug_struct("MultipliedStatement")
                .field("multiplier", multiplier)
                .field("element", element)
                .finish(),
            AstStatement::CaseCondition { condition, .. } => f
                .debug_struct("CaseCondition")
                .field("condition", condition)
                .finish(),
            AstStatement::ReturnStatement { .. } => f.debug_struct("ReturnStatement").finish(),
            AstStatement::ContinueStatement { .. } => f.debug_struct("ContinueStatement").finish(),
            AstStatement::ExitStatement { .. } => f.debug_struct("ExitStatement").finish(),
            AstStatement::CastStatement {
                target, type_name, ..
            } => f
                .debug_struct("CastStatement")
                .field("type_name", type_name)
                .field("target", target)
                .finish(),
        }
    }
}

impl AstStatement {
    ///Returns the statement in a singleton list, or the contained statements if the statement is already a list
    pub fn get_as_list(&self) -> Vec<&AstStatement> {
        if let AstStatement::ExpressionList { expressions, .. } = self {
            expressions.iter().collect::<Vec<&AstStatement>>()
        } else {
            vec![self]
        }
    }
    pub fn get_location(&self) -> SourceRange {
        match self {
            AstStatement::EmptyStatement { location, .. } => location.clone(),
            AstStatement::LiteralNull { location, .. } => location.clone(),
            AstStatement::LiteralInteger { location, .. } => location.clone(),
            AstStatement::LiteralDate { location, .. } => location.clone(),
            AstStatement::LiteralDateAndTime { location, .. } => location.clone(),
            AstStatement::LiteralTimeOfDay { location, .. } => location.clone(),
            AstStatement::LiteralTime { location, .. } => location.clone(),
            AstStatement::LiteralReal { location, .. } => location.clone(),
            AstStatement::LiteralBool { location, .. } => location.clone(),
            AstStatement::LiteralString { location, .. } => location.clone(),
            AstStatement::LiteralArray { location, .. } => location.clone(),
            AstStatement::Reference { location, .. } => location.clone(),
            AstStatement::QualifiedReference { elements, .. } => {
                let first = elements
                    .first()
                    .map_or_else(SourceRange::undefined, |it| it.get_location());
                let last = elements
                    .last()
                    .map_or_else(SourceRange::undefined, |it| it.get_location());
                SourceRange::new(first.get_start()..last.get_end())
            }
            AstStatement::BinaryExpression { left, right, .. } => {
                let left_loc = left.get_location();
                let right_loc = right.get_location();
                SourceRange::new(left_loc.range.start..right_loc.range.end)
            }
            AstStatement::UnaryExpression { location, .. } => location.clone(),
            AstStatement::ExpressionList { expressions, .. } => {
                let first = expressions
                    .first()
                    .map_or_else(SourceRange::undefined, |it| it.get_location());
                let last = expressions
                    .last()
                    .map_or_else(SourceRange::undefined, |it| it.get_location());
                SourceRange::new(first.get_start()..last.get_end())
            }
            AstStatement::RangeStatement { start, end, .. } => {
                let start_loc = start.get_location();
                let end_loc = end.get_location();
                SourceRange::new(start_loc.range.start..end_loc.range.end)
            }
            AstStatement::Assignment { left, right, .. } => {
                let left_loc = left.get_location();
                let right_loc = right.get_location();
                SourceRange::new(left_loc.range.start..right_loc.range.end)
            }
            AstStatement::OutputAssignment { left, right, .. } => {
                let left_loc = left.get_location();
                let right_loc = right.get_location();
                SourceRange::new(left_loc.range.start..right_loc.range.end)
            }
            AstStatement::CallStatement { location, .. } => location.clone(),
            AstStatement::IfStatement { location, .. } => location.clone(),
            AstStatement::ForLoopStatement { location, .. } => location.clone(),
            AstStatement::WhileLoopStatement { location, .. } => location.clone(),
            AstStatement::RepeatLoopStatement { location, .. } => location.clone(),
            AstStatement::CaseStatement { location, .. } => location.clone(),
            AstStatement::ArrayAccess {
                reference, access, ..
            } => {
                let reference_loc = reference.get_location();
                let access_loc = access.get_location();
                SourceRange::new(reference_loc.range.start..access_loc.range.end)
            }
            AstStatement::PointerAccess { reference, .. } => reference.get_location(),
            AstStatement::DirectAccess { location, .. } => location.clone(),
            AstStatement::HardwareAccess { location, .. } => location.clone(),
            AstStatement::MultipliedStatement { location, .. } => location.clone(),
            AstStatement::CaseCondition { condition, .. } => condition.get_location(),
            AstStatement::ReturnStatement { location, .. } => location.clone(),
            AstStatement::ContinueStatement { location, .. } => location.clone(),
            AstStatement::ExitStatement { location, .. } => location.clone(),
            AstStatement::CastStatement { location, .. } => location.clone(),
        }
    }

    pub fn get_id(&self) -> AstId {
        match self {
            AstStatement::EmptyStatement { id, .. } => *id,
            AstStatement::LiteralNull { id, .. } => *id,
            AstStatement::LiteralInteger { id, .. } => *id,
            AstStatement::LiteralDate { id, .. } => *id,
            AstStatement::LiteralDateAndTime { id, .. } => *id,
            AstStatement::LiteralTimeOfDay { id, .. } => *id,
            AstStatement::LiteralTime { id, .. } => *id,
            AstStatement::LiteralReal { id, .. } => *id,
            AstStatement::LiteralBool { id, .. } => *id,
            AstStatement::LiteralString { id, .. } => *id,
            AstStatement::LiteralArray { id, .. } => *id,
            AstStatement::MultipliedStatement { id, .. } => *id,
            AstStatement::QualifiedReference { id, .. } => *id,
            AstStatement::Reference { id, .. } => *id,
            AstStatement::ArrayAccess { id, .. } => *id,
            AstStatement::PointerAccess { id, .. } => *id,
            AstStatement::DirectAccess { id, .. } => *id,
            AstStatement::HardwareAccess { id, .. } => *id,
            AstStatement::BinaryExpression { id, .. } => *id,
            AstStatement::UnaryExpression { id, .. } => *id,
            AstStatement::ExpressionList { id, .. } => *id,
            AstStatement::RangeStatement { id, .. } => *id,
            AstStatement::Assignment { id, .. } => *id,
            AstStatement::OutputAssignment { id, .. } => *id,
            AstStatement::CallStatement { id, .. } => *id,
            AstStatement::IfStatement { id, .. } => *id,
            AstStatement::ForLoopStatement { id, .. } => *id,
            AstStatement::WhileLoopStatement { id, .. } => *id,
            AstStatement::RepeatLoopStatement { id, .. } => *id,
            AstStatement::CaseStatement { id, .. } => *id,
            AstStatement::CaseCondition { id, .. } => *id,
            AstStatement::ReturnStatement { id, .. } => *id,
            AstStatement::ContinueStatement { id, .. } => *id,
            AstStatement::ExitStatement { id, .. } => *id,
            AstStatement::CastStatement { id, .. } => *id,
        }
    }
    /// Returns true if the current statement has a return access.
    pub fn has_direct_access(&self) -> bool {
        if let AstStatement::QualifiedReference { elements, .. } = self {
            matches!(elements.last(), Some(AstStatement::DirectAccess { .. }))
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operator {
    Plus,
    Minus,
    Multiplication,
    Exponentiation,
    Division,
    Equal,
    NotEqual,
    Modulo,
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,
    Not,
    And,
    Or,
    Xor,
    Address,
}

impl Display for Operator {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let symbol = match self {
            Operator::Plus => "+",
            Operator::Minus => "-",
            Operator::Multiplication => "*",
            Operator::Division => "/",
            Operator::Equal => "=",
            Operator::Modulo => "MOD",
            Operator::Exponentiation => "**",
            _ => unimplemented!(),
        };
        f.write_str(symbol)
    }
}

/// enum_elements should be the statement between then enum's brackets ( )
/// e.g. x : ( this, that, etc)
pub fn get_enum_element_names(enum_elements: &AstStatement) -> Vec<String> {
    flatten_expression_list(enum_elements)
        .into_iter()
        .filter(|it| {
            matches!(
                it,
                AstStatement::Reference { .. } | AstStatement::Assignment { .. }
            )
        })
        .map(get_enum_element_name)
        .collect()
}

/// expects a Reference or an Assignment
pub fn get_enum_element_name(enum_element: &AstStatement) -> String {
    match enum_element {
        AstStatement::Reference { name, .. } => name.to_string(),
        AstStatement::Assignment { left, .. } => {
            if let AstStatement::Reference { name, .. } = left.as_ref() {
                name.to_string()
            } else {
                unreachable!("left of assignment not a reference")
            }
        }
        _ => {
            unreachable!(
                "expected {:?} to be a Reference or Assignment",
                enum_element
            );
        }
    }
}

/// flattens expression-lists and MultipliedStatements into a vec of statements.
/// It can also handle nested structures like 2(3(4,5))
pub fn flatten_expression_list(list: &AstStatement) -> Vec<&AstStatement> {
    match list {
        AstStatement::ExpressionList { expressions, .. } => expressions
            .iter()
            .by_ref()
            .flat_map(flatten_expression_list)
            .collect(),
        AstStatement::MultipliedStatement {
            multiplier,
            element,
            ..
        } => iter::repeat(flatten_expression_list(element))
            .take(*multiplier as usize)
            .flatten()
            .collect(),
        _ => vec![list],
    }
}

/// helper function that creates a call-statement
pub fn create_call_to(
    function_name: String,
    parameters: Vec<AstStatement>,
    function_id: usize,
    parameter_list_id: usize,
    location: &SourceRange,
) -> AstStatement {
    AstStatement::CallStatement {
        operator: Box::new(AstStatement::Reference {
            name: function_name,
            location: location.clone(),
            id: function_id,
        }),
        parameters: Box::new(Some(AstStatement::ExpressionList {
            expressions: parameters,
            id: parameter_list_id,
        })),
        location: location.clone(),
        id: function_id,
    }
}

/// helper function that creates an or-expression
pub fn create_or_expression(left: AstStatement, right: AstStatement) -> AstStatement {
    AstStatement::BinaryExpression {
        id: left.get_id(),
        left: Box::new(left),
        right: Box::new(right),
        operator: Operator::Or,
    }
}

/// helper function that creates an not-expression
pub fn create_not_expression(operator: AstStatement, location: SourceRange) -> AstStatement {
    AstStatement::UnaryExpression {
        id: operator.get_id(),
        value: Box::new(operator),
        location,
        operator: Operator::Not,
    }
}

pub fn create_reference(name: &str, location: &SourceRange, id: AstId) -> AstStatement {
    AstStatement::Reference {
        id,
        location: location.clone(),
        name: name.to_string(),
    }
}

pub fn create_literal_int(value: i128, location: &SourceRange, id: AstId) -> AstStatement {
    AstStatement::LiteralInteger {
        id,
        location: location.clone(),
        value,
    }
}

pub fn create_binary_expression(
    left: AstStatement,
    operator: Operator,
    right: AstStatement,
    id: AstId,
) -> AstStatement {
    AstStatement::BinaryExpression {
        id,
        left: Box::new(left),
        operator,
        right: Box::new(right),
    }
}

pub fn create_cast_statement(
    type_name: &str,
    stmt: AstStatement,
    location: &SourceRange,
    id: AstId,
) -> AstStatement {
    AstStatement::CastStatement {
        id,
        location: location.clone(),
        type_name: type_name.to_string(),
        target: Box::new(stmt),
    }
}

pub fn pre_process(unit: &mut CompilationUnit, id_provider: IdProvider) {
    pre_processor::pre_process(unit, id_provider)
}
impl Operator {
    /// returns true, if this operator results in a bool value
    pub(crate) fn is_bool_type(&self) -> bool {
        matches!(
            self,
            Operator::Equal
                | Operator::NotEqual
                | Operator::Less
                | Operator::Greater
                | Operator::LessOrEqual
                | Operator::GreaterOrEqual
        )
    }

    /// returns true, if this operator is a comparison operator
    /// (=, <>, >, <, >=, <=)
    pub(crate) fn is_comparison_operator(&self) -> bool {
        matches!(
            self,
            Operator::Equal
                | Operator::NotEqual
                | Operator::Less
                | Operator::Greater
                | Operator::LessOrEqual
                | Operator::GreaterOrEqual
        )
    }
}
