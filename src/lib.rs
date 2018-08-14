// Copyright (c) 2018 Jason Graalum //
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
// associated documentation files (the "Software"), to deal in the Software without restriction, // including without limitation the rights to use, copy, modify, merge, publish, distribute,
// sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or
// substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
// BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, // OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE. //
// web_map library
//
// Defines a structure to reflect the hierarchical traits of a web site
//
// Starts at a root node
// Includes References - <a hrefs ...> for now
// Includes Resources - <src img ...> for now

extern crate html5ever;
extern crate reqwest;
extern crate url;

pub mod tokenizer;

//use std::io;
use std::default::Default;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use self::tokenizer::UrlTokenParser;

use url::{Host, Url};

use reqwest::{Client, StatusCode};

use html5ever::tokenizer::BufferQueue;
use html5ever::tokenizer::Tokenizer;
use html5ever::tokenizer::TokenizerOpts;
use html5ever::tendril::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WebMap {
    hosts: Vec<(String, StatusCode, Url)>,
    resources: HashMap<u64, WebResource>,
    references: HashMap<u64, WebReference>,
    ref_tag_attr_pairs: Vec<(String, String)>,
    src_tag_attr_pairs: Vec<(String, String)>,
}

impl WebMap {
    // Create new web_map
    pub fn new() -> WebMap {
        WebMap {
            hosts: Vec::new(),
            references: HashMap::new(),
            resources: HashMap::new(),
            ref_tag_attr_pairs: Vec::new(),
            src_tag_attr_pairs: Vec::new(),
        }
    }

    pub fn add_host(&mut self, hostname: &str) -> bool {
        match Url::parse(hostname) {
            Err(_) => false,
            Ok(url) => {
                let mut hostname_string = String::new();
                hostname_string.push_str(hostname);

                let client = Client::new();
                let result_resp = client.get(url).send();
                match result_resp {
                    Ok(result) => {
                        self.hosts
                            .push((hostname_string, result.status(), result.url().clone()));
                        true
                    }
                    Err(_) => false,
                }
            }
        }
    }

    pub fn list_hosts(&self) -> Vec<String> {
        let mut host_list: Vec<String> = Vec::new();
        for &(ref h, ref _status, ref url) in &self.hosts {
            host_list.push(h.clone());
        }

        return host_list;
    }

    pub fn list_resources(&self) -> Vec<String> {
        let mut res_list: Vec<String> = Vec::new();
        //resources : HashMap<u64, WebResource>,
        for r in &self.resources {
            res_list.push(r.1.url.to_string().clone());
        }

        return res_list;
    }

    pub fn add_path(&mut self, hostname: &str, path: &str) -> u64 {
        {
            let mut full_url: Url;
            // Generate new reference hash for hostname/url combination
            if let Ok(base_url) = Url::parse(hostname) {
                if let Ok(path_url) = Url::parse(path) {
                    if path_url.cannot_be_a_base() {
                        match base_url.join(path) {
                            Ok(u) => full_url = u,
                            Err(_) => {
                                println!("Error 1");
                                return 0;
                            }
                        };
                    } else {
                        full_url = path_url;
                    }
                } else {
                    match base_url.join(path) {
                        Ok(u) => full_url = u,
                        Err(_) => {
                            println!("Error 1");
                            return 0;
                        }
                    };
                }
            } else {
                println!("Error 3");
                return 0;
            };

            let full_url_copy = full_url.clone();

            let mut hasher = DefaultHasher::new();
            full_url.to_string().hash(&mut hasher);
            let hash_val = hasher.finish();

            // Check if WebMap references HashMap contains this hash
            if self.references.contains_key(&hash_val) {
                return 0;
            };

            let client = Client::new();
            if let Ok(mut resp) = client.get(full_url).send() {
                if let Ok(resp_text) = resp.text() {


                    //println!("{:?}", resp_text);
                    let mut sink = UrlTokenParser {
                        in_char_run: false,
                        resources: Vec::new(),
                        references: Vec::new(),
                    };

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

                    let mut ref_urls: Vec<Url> = Vec::new();
                    let mut res_urls: Vec<Url> = Vec::new();
                    let mut ref_hashes: Vec<u64> = Vec::new();
                    let mut res_hashes: Vec<u64> = Vec::new();


                    for ref_str in tok.sink.references {
                        //println!("Ref: {:?}", ref_str);
                        match WebMap::validate_url_string(hostname, &ref_str) {
                            Some((ref_url, ref_hash)) => {
                                let new_ref: WebReference = WebReference::new(ref_url);
                                self.references.insert(ref_hash, new_ref);
                                ref_hashes.push(ref_hash);
                            }
                            None => {}
                        }
                    }

                    for res_str in tok.sink.resources {
                        //println!("Res: {:?}", res_str);
                        match WebMap::validate_url_string(hostname, &res_str) {
                            Some((res_url, res_hash)) => {
                                let new_res: WebResource = WebResource { url: res_url, resource_type: String::new() } ;
                                self.resources.insert(res_hash, new_res);
                                res_hashes.push(res_hash);
                            }
                            None => {}
                        }
                    }

                    let new_node: WebReference = WebReference {
                        url: resp.url().to_owned(),
                        status: Option::from(resp.status()),
                        resources: res_hashes,
                        references: ref_hashes,
                        children: Vec::new(),
                    };

                    // Insert this path in to the references hash
                    self.references.insert(hash_val, new_node);

                    return hash_val;
                } else {
                    println!("Error 4");
                    return 0;
                }
            } else {
                println!("Error 5");
                return 0;
            }
        }
    }

    pub fn hash_host_and_url(hostname: &str, url_name: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        url_name.hash(&mut hasher);
        hostname.hash(&mut hasher);
        hasher.finish()
    }

    pub fn validate_url_string(base_name: &str, url: &str) -> Option<(Url, u64)> {
        match Url::parse(url) {
            Err(_) => match Url::parse(base_name) {
                Err(_) => return None,
                Ok(base_url) => match base_url.join(url) {
                    Err(_) => return None,
                    Ok(final_url) => {
                        let hash = WebMap::hash_host_and_url(base_name, final_url.as_str());
                        return Some((final_url, hash));
                    }
                },
            },
            Ok(final_url) => {
                let hash = WebMap::hash_host_and_url(base_name, final_url.as_str());
                return Some((final_url, hash));
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WebReference {
    url: Url,
    status: Option<StatusCode>,
    references: Vec<u64>,
    resources: Vec<u64>,
    children: Vec<u64>,
}

impl Hash for WebReference {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let host = self.url.host_str();
        let path = self.url.path();
        host.hash(state);
        path.hash(state);
        self.status.hash(state);
    }
}

impl WebReference {
    pub fn new(url: Url) -> WebReference {
        // GET Response
        WebReference {
            url: url,
            references: Vec::new(),
            resources: Vec::new(),
            status: None,
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebResource {
    url: Url,
    resource_type: String,
}

impl Hash for WebResource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let host = self.url.host_str();
        let path = self.url.path();
        host.hash(state);
        path.hash(state);
        self.resource_type.hash(state);
    }
}

#[test]
fn webmap_add_new_host() {
    let mut map = WebMap::new();
    if map.add_host("https://www.pdx.edu") == true && map.add_host("https://www.google.com") == true
    {
        for h in map.list_hosts() {
            println!("Host : {:?}", h);
        }
        assert!(true);
    } else {
        assert!(false);
    }
}

#[test]
fn test_local() {
    let mut map = WebMap::new();
    if map.add_host("file:///User/jasongraalum") == true {
        for h in map.list_hosts() {
            println!("Host : {:?}", h);
        }
        assert!(true);
    } else {
        assert!(false);
    }
}

#[test]
fn test_image_grab() {
    let mut map = WebMap::new();
    map.add_host("https://web.cecs.pdx.edu/~jgraalum/current/jgraalum_pdxedu_site.html");
    map.add_path(
        "https://web.cecs.pdx.edu/",
        "~jgraalum/current/jgraalum_pdxedu_site.html",
    );

    println!("Resources:");
    assert_eq!(map.list_resources(), vec!["https://web.cecs.pdx.edu/images/D&J%20Blazers.jpg"]);
}
