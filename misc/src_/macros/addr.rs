macro_rules! give_attr {
    ($($decl:item)*) => {
        $(
            #[repr(C)]
            #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
            $decl
        )*
    };
}
