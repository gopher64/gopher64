use crate::Encoding;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NestingLevel {
    Top,
    Within,
    Bottom,
}

impl NestingLevel {
    pub(crate) const fn new() -> Self {
        Self::Top
    }

    pub(crate) const fn bitfield(self) -> Self {
        // TODO: Is this correct?
        self
    }

    pub(crate) const fn indirection(self, kind: IndirectionKind) -> Self {
        match kind {
            // Move all the way down
            IndirectionKind::Atomic => Self::Bottom,
            // Move one step down
            IndirectionKind::Pointer => match self {
                Self::Top => Self::Within,
                Self::Bottom | Self::Within => Self::Bottom,
            },
        }
    }

    pub(crate) const fn array(self) -> Self {
        // TODO: Is this correct?
        self
    }

    pub(crate) const fn container(self) -> Option<Self> {
        match self {
            // Move one step down, and output
            Self::Top => Some(Self::Within),
            // Output
            Self::Within => Some(Self::Within),
            // Don't output
            Self::Bottom => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) enum Primitive {
    Char,
    Short,
    Int,
    Long,
    LongLong,
    UChar,
    UShort,
    UInt,
    ULong,
    ULongLong,
    Float,
    Double,
    LongDouble,
    FloatComplex,
    DoubleComplex,
    LongDoubleComplex,
    Bool,
    Void,
    String,
    Object,
    Block,
    Class,
    Sel,
    Unknown,
}

impl Primitive {
    pub(crate) const fn to_str(self) -> &'static str {
        use Primitive::*;
        match self {
            Char => "c",
            Short => "s",
            Int => "i",
            Long => "l",
            LongLong => "q",
            UChar => "C",
            UShort => "S",
            UInt => "I",
            ULong => "L",
            ULongLong => "Q",
            Float => "f",
            Double => "d",
            LongDouble => "D",
            FloatComplex => "jf",
            DoubleComplex => "jd",
            LongDoubleComplex => "jD",
            Bool => "B",
            Void => "v",
            String => "*",
            Object => "@",
            Block => "@?",
            Class => "#",
            Sel => ":",
            Unknown => "?",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum IndirectionKind {
    Atomic,
    Pointer,
}

impl IndirectionKind {
    pub(crate) const fn prefix(self) -> char {
        self.prefix_byte() as char
    }

    pub(crate) const fn prefix_byte(self) -> u8 {
        match self {
            Self::Atomic => b'A',
            Self::Pointer => b'^',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ContainerKind {
    Struct,
    Union,
}

impl ContainerKind {
    pub(crate) const fn start(self) -> char {
        self.start_byte() as char
    }

    pub(crate) const fn end(self) -> char {
        self.end_byte() as char
    }

    pub(crate) const fn start_byte(self) -> u8 {
        match self {
            Self::Struct => b'{',
            Self::Union => b'(',
        }
    }

    pub(crate) const fn end_byte(self) -> u8 {
        match self {
            Self::Struct => b'}',
            Self::Union => b')',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) enum Helper<'a> {
    Primitive(Primitive),
    BitField(u8, &'a Encoding),
    Indirection(IndirectionKind, &'a Encoding),
    Array(usize, &'a Encoding),
    Container(ContainerKind, &'a str, &'a [Encoding]),
}

impl<'a> Helper<'a> {
    pub(crate) const fn new(encoding: &'a Encoding) -> Self {
        use Encoding::*;
        match encoding {
            Char => Self::Primitive(Primitive::Char),
            Short => Self::Primitive(Primitive::Short),
            Int => Self::Primitive(Primitive::Int),
            Long => Self::Primitive(Primitive::Long),
            LongLong => Self::Primitive(Primitive::LongLong),
            UChar => Self::Primitive(Primitive::UChar),
            UShort => Self::Primitive(Primitive::UShort),
            UInt => Self::Primitive(Primitive::UInt),
            ULong => Self::Primitive(Primitive::ULong),
            ULongLong => Self::Primitive(Primitive::ULongLong),
            Float => Self::Primitive(Primitive::Float),
            Double => Self::Primitive(Primitive::Double),
            LongDouble => Self::Primitive(Primitive::LongDouble),
            FloatComplex => Self::Primitive(Primitive::FloatComplex),
            DoubleComplex => Self::Primitive(Primitive::DoubleComplex),
            LongDoubleComplex => Self::Primitive(Primitive::LongDoubleComplex),
            Bool => Self::Primitive(Primitive::Bool),
            Void => Self::Primitive(Primitive::Void),
            String => Self::Primitive(Primitive::String),
            Object => Self::Primitive(Primitive::Object),
            Block => Self::Primitive(Primitive::Block),
            Class => Self::Primitive(Primitive::Class),
            Sel => Self::Primitive(Primitive::Sel),
            Unknown => Self::Primitive(Primitive::Unknown),
            BitField(b, t) => Self::BitField(*b, t),
            Pointer(t) => Self::Indirection(IndirectionKind::Pointer, t),
            Atomic(t) => Self::Indirection(IndirectionKind::Atomic, t),
            Array(len, item) => Self::Array(*len, item),
            Struct(name, fields) => Self::Container(ContainerKind::Struct, name, fields),
            Union(name, members) => Self::Container(ContainerKind::Union, name, members),
        }
    }
}
