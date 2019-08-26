use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::model::{
    channel::{ChannelType, PermissionOverwrite, PermissionOverwriteType::Role},
    id::{ChannelId, GuildId, RoleId},
    permissions::Permissions,
};
use serenity::prelude::Context;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::{BufReader, BufWriter};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MiEI {
    courses: HashMap<String, Year>,
}

impl MiEI {
    fn write_courses(&self) -> Result<(), io::Error> {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open("config.json")?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &self)?;
        Ok(())
    }

    pub fn get_role_id<'a>(&'a self, role_name: &'a str) -> Vec<(&'a str, RoleId)> {
        let years = &self.courses;
        lazy_static! {
            static ref REGEX: Regex =
                Regex::new("([0-9]+)(?i)ano([0-9]+)((?i)semestre|sem)").unwrap();
            static ref YEAR_REGEX: Regex = Regex::new("([0-9])+((?i)ano)").unwrap();
        };
        if let Some(splits) = REGEX.captures(role_name) {
            match years.get(&splits[1]) {
                Some(x) => x.get_semester_roles(&splits[2]),
                None => Vec::new(),
            }
        } else if let Some(splits) = YEAR_REGEX.captures(role_name) {
            match years.get(&splits[1]) {
                Some(x) => x.get_year_roles(),
                None => Vec::new(),
            }
        } else {
            years
                .values()
                .flat_map(|x| x.get_role(&role_name.to_uppercase()))
                .map(|x| (role_name, x))
                .collect::<Vec<(&str, RoleId)>>()
        }
    }

    pub fn create_role<'a>(
        &mut self,
        ctx: &Context,
        year: &str,
        semester: &str,
        course: &'a str,
        guild: GuildId,
    ) -> Option<&'a str> {
        let upper_course = course.to_uppercase();
        if self.role_exists(&upper_course) {
            None
        } else {
            let role = guild
                .create_role(&ctx.http, |z| {
                    z.hoist(false).mentionable(true).name(&upper_course)
                })
                .unwrap();
            let perms = vec![
                PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::READ_MESSAGES,
                    kind: Role(guild.as_u64().to_owned().into()),
                },
                PermissionOverwrite {
                    allow: Permissions::READ_MESSAGES,
                    deny: Permissions::empty(),
                    kind: Role(role.id),
                },
            ];
            let category = guild
                .create_channel(&ctx, |c| {
                    c.name(&upper_course)
                        .kind(ChannelType::Category)
                        .permissions(perms)
                })
                .unwrap();
            let anexos = guild
                .create_channel(&ctx, |c| {
                    c.name(format!("anexos-{}", &upper_course))
                        .kind(ChannelType::Text)
                        .category(category.id)
                })
                .unwrap();
            let duvidas = guild
                .create_channel(&ctx, |c| {
                    c.name(format!("duvidas-{}", &upper_course))
                        .kind(ChannelType::Text)
                        .category(category.id)
                })
                .unwrap();
            let courses = Course {
                role: role.id,
                channels: vec![category.id, anexos.id, duvidas.id],
            };
            self.add_role(&upper_course, courses, semester, year);
            match self.write_courses() {
                Ok(()) => (),
                Err(b) => panic!("{}", b),
            };
            Some(course)
        }
    }

    fn add_role(&mut self, role_name: &str, course: Course, semester: &str, year: &str) {
        self.courses
            .entry(year.to_string())
            .or_insert(Year {
                courses: HashMap::new(),
            })
            .add_role(role_name, course, semester);
    }

    fn role_exists(&self, role_name: &str) -> bool {
        self.courses.values().any(|x| x.role_exists(role_name))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Year {
    #[serde(flatten)]
    courses: HashMap<String, Semester>,
}

impl Year {
    fn role_exists(&self, role_name: &str) -> bool {
        self.courses
            .values()
            .any(|x| x.courses.contains_key(role_name))
    }

    fn get_semester_roles(&self, semester: &str) -> Vec<(&str, RoleId)> {
        match self.courses.get(semester) {
            Some(x) => x
                .courses
                .iter()
                .map(|(a, z)| (a.as_str(), z.role))
                .collect::<Vec<(&str, RoleId)>>(),
            None => Vec::new(),
        }
    }

    fn get_year_roles(&self) -> Vec<(&str, RoleId)> {
        self.courses
            .values()
            .flat_map(|x| x.courses.iter().map(|(a, z)| (a.as_str(), z.role)))
            .collect::<Vec<(&str, RoleId)>>()
    }

    fn get_role<'a>(&self, role_name: &'a str) -> Option<RoleId> {
        self.courses
            .values()
            .filter_map(|x| x.courses.get(role_name))
            .map(|x| x.role)
            .next()
    }

    fn add_role(&mut self, role_name: &str, course: Course, semester: &str) {
        self.courses
            .entry(semester.to_string())
            .or_insert(Semester {
                courses: HashMap::new(),
            })
            .courses
            .insert(role_name.to_string(), course);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Semester {
    #[serde(flatten)]
    courses: HashMap<String, Course>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Course {
    role: RoleId,
    channels: Vec<ChannelId>,
}

pub fn read_courses() -> io::Result<MiEI> {
    let file = File::open("config.json")?;
    let reader = BufReader::new(file);

    let u = serde_json::from_reader(reader)?;
    Ok(u)
}