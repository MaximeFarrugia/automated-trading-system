use std::borrow::Cow;

use url::Url;

#[derive(Debug, Default)]
pub struct QueryParams<'a> {
    params: Vec<(Cow<'a, str>, Cow<'a, str>)>,
}

impl<'a> QueryParams<'a> {
    pub fn push<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<Cow<'a, str>>,
        V: Into<Cow<'a, str>>,
    {
        self.params.push((key.into(), value.into()));

        return self;
    }

    pub fn push_opt<K, V>(&mut self, key: K, value: Option<V>) -> &mut Self
    where
        K: Into<Cow<'a, str>>,
        V: Into<Cow<'a, str>>,
    {
        if let Some(value) = value {
            self.params.push((key.into(), value.into()));
        }

        return self;
    }

    pub fn extend<I, K, V>(&mut self, iter: I) -> &mut Self
    where
        I: Iterator<Item = (K, V)>,
        K: Into<Cow<'a, str>>,
        V: Into<Cow<'a, str>>,
    {
        self.params
            .extend(iter.map(|(key, value)| (key.into().into(), value.into().into())));

        return self;
    }

    pub fn add_to_url(&self, url: &mut Url) {
        url.query_pairs_mut().extend_pairs(self.params.iter());
    }
}
