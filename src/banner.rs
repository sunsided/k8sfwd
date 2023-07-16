// SPDX-FileCopyrightText: Copyright 2023 Markus Mayer
// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileType: SOURCE

use std::fmt::{Display, Formatter};

static BANNER: &'static str = indoc::indoc!(
    r#"
    ██╗░░██╗░█████╗░░██████╗░░░░░███████╗██╗░░░░░░░██╗██████╗
    ██║░██╔╝██╔══██╗██╔════╝░██╗░██╔════╝██║░░██╗░░██║██╔══██╗
    █████═╝░╚█████╔╝╚█████╗░░╚═╝░█████╗░░╚██╗████╗██╔╝██║░░██║
    ██╔═██╗░██╔══██╗░╚═══██╗░██╗░██╔══╝░░░████╔═████║░██║░░██║
    ██║░╚██╗╚█████╔╝██████╔╝░╚═╝░██║░░░░░░╚██╔╝░╚██╔╝░██████╔╝
    ╚═╝░░╚═╝░╚════╝░╚═════╝░░░░░░╚═╝░░░░░░░╚═╝░░░╚═╝░░╚═════╝"#
);

pub struct Banner;

impl Banner {
    pub fn println() {
        println!("{BANNER}")
    }
}

impl Display for Banner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{BANNER}")
    }
}
