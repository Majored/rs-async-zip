// Copyright (c) 2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use tokio_util::compat::Compat;

#[cfg(feature = "tokio-fs")]
pub mod fs;

pub type ZipEntryReader<'a, R> = crate::base::read::ZipEntryReader<'a, Compat<R>>;

pub mod seek {
    use tokio_util::compat::Compat;

    pub type ZipFileReader<R> = crate::base::read::seek::ZipFileReader<Compat<R>>;
}

pub mod stream {
    use tokio_util::compat::Compat;

    pub type Reading<'a, R> = crate::base::read::stream::Reading<'a, Compat<R>>;
    pub type Ready<R> = crate::base::read::stream::Ready<Compat<R>>;
}
