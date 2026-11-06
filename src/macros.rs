macro_rules! int_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident($repr:ty) {
            $( $(#[$vmeta:meta])* $variant:ident = $value:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[non_exhaustive]
        $vis enum $name {
            $( $(#[$vmeta])* #[doc = concat!("Wire value `", stringify!($value), "`.")] $variant ),+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The value used on the wire.
            pub const fn value(self) -> $repr {
                match self { $(Self::$variant => $value),+ }
            }

            /// The variant for a wire value, if there is one.
            pub const fn from_value(value: $repr) -> Option<Self> {
                match value { $($value => Some(Self::$variant),)+ _ => None }
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                ::serde::Serialize::serialize(&self.value(), serializer)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = <$repr as ::serde::Deserialize>::deserialize(deserializer)?;
                Self::from_value(value).ok_or_else(|| {
                    <D::Error as ::serde::de::Error>::custom(format!(
                        concat!("unknown ", stringify!($name), " value {}"),
                        value
                    ))
                })
            }
        }
    };
}

macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident = $value:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[non_exhaustive]
        $vis enum $name {
            $( $(#[$vmeta])* #[doc = concat!("Wire value `", $value, "`.")] $variant ),+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The value used on the wire.
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $value),+ }
            }

            /// The variant for a wire value, if there is one.
            pub fn from_name(name: &str) -> Option<Self> {
                match name { $($value => Some(Self::$variant),)+ _ => None }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let name = <String as ::serde::Deserialize>::deserialize(deserializer)?;
                Self::from_name(&name).ok_or_else(|| {
                    <D::Error as ::serde::de::Error>::custom(format!(
                        concat!("unknown ", stringify!($name), " value `{}`"),
                        name
                    ))
                })
            }
        }
    };
}

macro_rules! bounded_string {
    (
        $(#[$meta:meta])*
        $name:ident, $field:literal, $min:expr, $max:expr $(, $check:expr)?
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ::serde::Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Validates and wraps a value.
            pub fn new(value: impl Into<String>) -> Result<Self, $crate::error::ValidationError> {
                let value = value.into();
                let length = value.chars().count();
                if !($min..=$max).contains(&length) {
                    return Err($crate::error::ValidationError::new(
                        $field,
                        format!("length {length} is outside {}..={}", $min, $max),
                    ));
                }
                $(
                    let check: fn(&str) -> Option<&'static str> = $check;
                    if let Some(reason) = check(&value) {
                        return Err($crate::error::ValidationError::new($field, reason));
                    }
                )?
                Ok(Self(value))
            }

            /// The validated value.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::error::ValidationError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::new(s)
            }
        }

        impl TryFrom<String> for $name {
            type Error = $crate::error::ValidationError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = $crate::error::ValidationError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
            ::serde::Serialize, ::serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub u64);

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl From<u64> for $name {
            fn from(value: u64) -> Self {
                Self(value)
            }
        }
    };
}
