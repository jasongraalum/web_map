// Copyright 2014-2017 The html5ever Project Developers. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
// Modifications covered under MIT License(below)
//
// Copyright 2018 Jason Graalum
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
// associated documentation files (the "Software"), to deal in the Software without restriction,
// including without limitation the rights to use, copy, modify, merge, publish, distribute,
// sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or
// substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
// BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// This is a heavily modified version of the html5ever/examples/tokenizer.rs file.

extern crate html5ever;

use std::io;
use std::default::Default;

use html5ever::tokenizer::{ParseError, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts};
use html5ever::tokenizer::{CharacterTokens, EndTag, NullCharacterToken, StartTag, TagToken};
use html5ever::tokenizer::BufferQueue;
use html5ever::tendril::*;

use reqwest;

#[derive(Clone)]
pub struct UrlTokenParser {
    pub in_char_run: bool,
    pub resources: Vec<String>,
    pub references: Vec<String>,
}

impl TokenSink for UrlTokenParser {
    type Handle = ();

    fn process_token(&mut self, token: Token, _line_number: u64) -> TokenSinkResult<()> {
        match token {
            TagToken(tag) => {
                for attr in tag.attrs.iter() {
                    if attr.name.local == "href".get(0..).unwrap().to_string() {
                        self.references
                            .push(attr.value.get(0..).unwrap().to_string());
                    }
                    /*
                    // TODO: Need to add ability to find permanently moved resources
                    if attr.name.local == "url".get(0..).unwrap().to_string(){
                        self.references.push(attr.value.get(0..).unwrap().to_string());
                    }
                    */
                    if attr.name.local.get(0..).unwrap().to_string() == "src" {
                        self.resources
                            .push(attr.value.get(0..).unwrap().to_string());
                    }
                }
            }
            _ => {
                //println!("OTHER: {:?}", token);
            }
        }
        TokenSinkResult::Continue
    }
}

#[test]
fn test_tokenizer() {
    let sink = UrlTokenParser {
        in_char_run: false,
        resources: Vec::new(),
        references: Vec::new(),
    };
    let mut resp_text = reqwest::get("https://web.cecs.pdx.edu/~jgraalum")
        .unwrap()
        .text()
        .unwrap();

    let mut chunk = ByteTendril::new();
    chunk.try_push_bytes(resp_text.as_bytes()).unwrap();

    let mut input = BufferQueue::new();
    input.push_back(chunk.try_reinterpret().unwrap());

    let mut tok = Tokenizer::new(
        sink,
        TokenizerOpts {
            profile: true,
            ..Default::default()
        },
    );

    let _ = tok.feed(&mut input);
    assert!(input.is_empty());

    tok.end();
    println!("References");
    println!("{:?}", tok.sink.references);
    println!("Resources");
    println!("{:?}", tok.sink.resources);
}
