use std::{env, vec};

use ::entities::{prelude::*, *};
use reqwest::StatusCode;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::{chrono::Utc, Uuid};

pub struct Chapters;

#[derive(Serialize)]
struct MuxBody {
    input: String,
    playback_policy: Vec<String>,
    video_quality: String,
}

#[derive(Deserialize, Debug)]
struct MuxResponse {
    data: MuxStatusAndData,
}

#[derive(Deserialize, Debug)]
struct MuxStatusAndData {
    status: String,
    playback_ids: Vec<PlayBackData>,
    mp4_support: String,
    master_access: String,
    id: String,
    encoding_tier: String,
    video_quality: String,
    created_at: String,
}

#[derive(Deserialize, Debug)]
struct PlayBackData {
    policy: String,
    id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromQueryResult)]
struct ChapterPrice {
    price: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChapterDetails {
    chapter: Option<chapter::Model>,
    course_price: Option<ChapterPrice>,
    mux_data: Option<mux_data::Model>,
    attachments: Option<Vec<attachment::Model>>,
    next_chapter: Option<chapter::Model>,
    user_progress: Option<user_progress::Model>,
    purchase: Option<purchase::Model>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReorderData {
    id: String,
    position: i32,
}

impl Chapters {
    pub async fn get(
        db: &DbConn,
        user_id: String,
        course_id: String,
        chapter_id: String,
    ) -> Result<ChapterDetails, DbErr> {
        let purchase = purchase::Entity::find()
            .filter(purchase::Column::UserId.eq(user_id.clone())) // Filter by userId
            .filter(purchase::Column::CourseId.eq(course_id.clone())) // Filter by courseId
            .one(db) // Retrieve one record
            .await?;

        let course_price = Course::find_by_id(course_id.clone())
            .filter(course::Column::IsPublished.eq(true))
            .select_only()
            .column(course::Column::Price)
            .into_model::<ChapterPrice>()
            .one(db)
            .await?;

        let chapter = Chapter::find_by_id(chapter_id.clone())
            .filter(chapter::Column::IsPublished.eq(true))
            .one(db)
            .await?;

        if chapter.is_none() || course_price.is_none() {
            return Err(DbErr::Custom("Chapter or course not found".into()));
        }

        let chapter = chapter.unwrap();

        let mut mux_data: Option<mux_data::Model> = None;
        let mut attachments: Option<Vec<attachment::Model>> = Some(Vec::new());
        let mut next_chapter: Option<chapter::Model> = None;

        if purchase.is_some() {
            attachments = Some(
                Attachment::find()
                    .filter(attachment::Column::CourseId.eq(course_id.clone()))
                    .all(db)
                    .await?,
            );
        }

        if chapter.is_free || purchase.is_some() {
            mux_data = MuxData::find()
                .filter(mux_data::Column::ChapterId.eq(chapter_id.clone()))
                .one(db)
                .await?;

            next_chapter = Chapter::find()
                .filter(chapter::Column::CourseId.eq(course_id.clone()))
                .filter(chapter::Column::IsPublished.eq(true))
                .filter(chapter::Column::Position.gt(chapter.position))
                .order_by_asc(chapter::Column::Position)
                .one(db)
                .await?;
        }

        let user_progress = UserProgress::find()
            .filter(user_progress::Column::ChapterId.eq(chapter_id.clone()))
            .filter(user_progress::Column::UserId.eq(user_id.clone()))
            .one(db)
            .await?;

        Ok(ChapterDetails {
            chapter: Some(chapter),
            course_price,
            mux_data,
            attachments,
            next_chapter,
            user_progress,
            purchase,
        })
    }

    pub async fn delete(
        db: &DbConn,
        user_id: String,
        course_id: String,
        chapter_id: String,
    ) -> Result<(), DbErr> {
        let client = reqwest::Client::new();
        let mux_token_id = env::var("MUX_TOKEN_ID").expect("Cannot get Mux token id");
        let mux_token_secret = env::var("MUX_TOKEN_SECRET").expect("Cannot get Mux token secret");

        let owned_course = Course::find_by_id(course_id.clone())
            .filter(course::Column::UserId.eq(user_id.clone()))
            .one(db)
            .await?;

        match owned_course {
            Some(_) => (),
            _ => return Err(DbErr::Custom("Cannot find course".into())),
        };

        let chapter = Chapter::find_by_id(chapter_id.clone())
            .filter(chapter::Column::CourseId.eq(course_id.clone()))
            .one(db)
            .await?;

        let chapter = match chapter {
            Some(chapter) => chapter,
            _ => return Err(DbErr::Custom("Cannot find chapter".into())),
        };

        if chapter.video_url.is_some() {
            let existing_mux_data = MuxData::find()
                .filter(mux_data::Column::ChapterId.eq(chapter_id.clone()))
                .one(db)
                .await?;

            if existing_mux_data.is_some() {
                let mux_data_id = existing_mux_data.unwrap().clone().asset_id;
                let _ = client
                    .delete(format!(
                        "https://api.mux.com/video/v1/assets/{}",
                        mux_data_id.clone()
                    ))
                    .basic_auth(mux_token_id.clone(), Some(mux_token_secret.clone()))
                    .send();

                let delete_mux_data = MuxData::find_by_id(mux_data_id.clone())
                    .one(db)
                    .await?
                    .unwrap();

                delete_mux_data.delete(db).await?;
            }
        }

        chapter.delete(db).await?;

        let published_chapters_in_course = Chapter::find()
            .filter(chapter::Column::CourseId.eq(course_id.clone()))
            .filter(chapter::Column::IsPublished.eq(true))
            .all(db)
            .await?;

        if published_chapters_in_course.len() == 0 {
            let mut update_course: course::ActiveModel = Course::find_by_id(course_id.clone())
                .one(db)
                .await?
                .unwrap()
                .into();

            update_course.is_published = Set(false);
            update_course.update(db).await?;
        }

        Ok(())
    }

    pub async fn publish(
        db: &DbConn,
        user_id: String,
        course_id: String,
        chapter_id: String,
    ) -> Result<(), DbErr> {
        let owned_course = Course::find_by_id(course_id.clone())
            .filter(course::Column::UserId.eq(user_id.clone()))
            .one(db)
            .await?;

        match owned_course {
            Some(_) => (),
            _ => return Err(DbErr::RecordNotFound("Cannot find course".into())),
        }

        let chapter = Chapter::find_by_id(chapter_id.clone())
            .filter(chapter::Column::CourseId.eq(course_id.clone()))
            .one(db)
            .await?;

        let chapter = match chapter {
            Some(chapter) => chapter,
            _ => return Err(DbErr::RecordNotFound("Cannot find chapter".into())),
        };

        let mux_data = MuxData::find()
            .filter(mux_data::Column::ChapterId.eq(chapter_id.clone()))
            .one(db)
            .await?;

        if mux_data.is_none() || chapter.description.is_none() || chapter.video_url.is_none() {
            return Err(DbErr::AttrNotSet("Missing required field".into()));
        }

        let mut chapter: chapter::ActiveModel = chapter.into();
        chapter.is_published = Set(true);
        chapter.update(db).await?;

        Ok(())
    }

    pub async fn unpublish(
        db: &DbConn,
        user_id: String,
        course_id: String,
        chapter_id: String,
    ) -> Result<(), DbErr> {
        let owned_course = Course::find_by_id(course_id.clone())
            .filter(course::Column::UserId.eq(user_id.clone()))
            .one(db)
            .await?;

        match owned_course {
            Some(_) => (),
            _ => return Err(DbErr::RecordNotFound("Cannot find course".into())),
        }

        let chapter = Chapter::find_by_id(chapter_id.clone())
            .filter(chapter::Column::CourseId.eq(course_id.clone()))
            .one(db)
            .await?;

        let mut chapter: chapter::ActiveModel = match chapter {
            Some(chapter) => chapter.into(),
            _ => return Err(DbErr::RecordNotFound("Cannot find chapter".into())),
        };
        chapter.is_published = Set(false);
        chapter.update(db).await?;

        let published_chapters_in_course = Chapter::find()
            .filter(chapter::Column::CourseId.eq(course_id.clone()))
            .filter(chapter::Column::IsPublished.eq(true))
            .all(db)
            .await?;

        if published_chapters_in_course.len() == 0 {
            let mut update_course: course::ActiveModel = Course::find_by_id(course_id.clone())
                .one(db)
                .await?
                .unwrap()
                .into();

            update_course.is_published = Set(false);
            update_course.update(db).await?;
        }

        Ok(())
    }

    pub async fn update_progress(
        db: &DbConn,
        user_id: String,
        chapter_id: String,
        is_completed: bool,
    ) -> Result<(), DbErr> {
        if let Some(existing_progress) = UserProgress::find()
            .filter(user_progress::Column::UserId.eq(user_id.clone()))
            .filter(user_progress::Column::ChapterId.eq(chapter_id.clone()))
            .one(db)
            .await?
        {
            let mut existing_progress: user_progress::ActiveModel = existing_progress.into();
            existing_progress.is_completed = Set(is_completed.into());
            existing_progress.updated_at = Set(Utc::now().naive_utc());
            existing_progress.update(db).await?;
        } else {
            let new_progress = user_progress::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                user_id: Set(user_id.clone()),
                chapter_id: Set(chapter_id.clone()),
                is_completed: Set(is_completed.clone()),
                created_at: Set(Utc::now().naive_utc()),
                updated_at: Set(Utc::now().naive_utc()),
                ..Default::default()
            };
            new_progress.insert(db).await?;
        }

        Ok(())
    }

    pub async fn reorder(
        db: &DbConn,
        user_id: String,
        course_id: String,
        list: Vec<ReorderData>,
    ) -> Result<Vec<chapter::Model>, DbErr> {
        let owned_course = Course::find_by_id(course_id.clone())
            .filter(course::Column::UserId.eq(user_id.clone()))
            .one(db)
            .await?;

        match owned_course {
            Some(_) => (),
            _ => return Err(DbErr::RecordNotFound("Cannot find course".into())),
        }

        for item in list.iter() {
            let mut update_chapter: chapter::ActiveModel = Chapter::find_by_id(item.id.clone())
                .one(db)
                .await?
                .unwrap()
                .into();

            update_chapter.position = Set(item.position.into());
            update_chapter.update(db).await?;
        }

        let chapter = Chapter::find()
            .filter(chapter::Column::CourseId.eq(course_id.clone()))
            .order_by_asc(chapter::Column::Position)
            .all(db)
            .await?;

        Ok(chapter)
    }

    pub async fn create(
        db: &DbConn,
        user_id: String,
        course_id: String,
        title: String,
    ) -> Result<Vec<chapter::Model>, DbErr> {
        let owned_coruse = Course::find_by_id(course_id.clone())
            .filter(course::Column::UserId.eq(user_id.clone()))
            .one(db)
            .await?;

        if owned_coruse.is_none() {
            return Err(DbErr::RecordNotFound("Cannot find course".into()));
        }

        let last_chapter = Chapter::find()
            .filter(chapter::Column::CourseId.eq(course_id.clone()))
            .order_by_asc(chapter::Column::Position)
            .one(db)
            .await?;

        let new_postion = match last_chapter {
            Some(chapter) => chapter.position + 1,
            _ => 1,
        };

        let chapter = chapter::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            title: Set(title.clone()),
            course_id: Set(course_id.clone()),
            position: Set(new_postion.clone()),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        };

        chapter.insert(db).await?;

        let res = Chapter::find()
            .filter(chapter::Column::CourseId.eq(course_id.clone()))
            .order_by_asc(chapter::Column::Position)
            .all(db)
            .await?;

        Ok(res)
    }

    pub async fn update(
        db: &DbConn,
        user_id: String,
        course_id: String,
        chapter_id: String,
        updates: std::collections::HashMap<String, Value>,
    ) -> Result<(), DbErr> {
        let owned_course = Course::find_by_id(course_id.clone())
            .filter(course::Column::UserId.eq(user_id.clone()))
            .one(db)
            .await?;

        match owned_course {
            Some(_) => (),
            _ => return Err(DbErr::RecordNotFound("Cannot find course".into())),
        }

        let mut chapter: chapter::ActiveModel = Chapter::find_by_id(chapter_id.clone())
            .filter(chapter::Column::CourseId.eq(course_id.clone()))
            .one(db)
            .await?
            .unwrap()
            .into();

        for (key, value) in updates.iter() {
            match key.as_str() {
                "isFree" => {
                    if let Some(is_free) = value.as_bool() {
                        chapter.is_free = Set(is_free);
                    }
                }
                "description" => {
                    if let Some(description) = value.as_str() {
                        chapter.description = Set(Some(description.to_string()));
                    }
                }
                "videoUrl" => {
                    if let Some(video_url) = value.as_str() {
                        let client = reqwest::Client::new();
                        let mux_token_id =
                            env::var("MUX_TOKEN_ID").expect("Cannot get Mux token id");
                        let mux_token_secret =
                            env::var("MUX_TOKEN_SECRET").expect("Cannot get Mux token secret");

                        chapter.video_url = Set(Some(video_url.to_string()));

                        let existing_mux_data = MuxData::find()
                            .filter(mux_data::Column::ChapterId.eq(chapter_id.clone()))
                            .one(db)
                            .await?;

                        if existing_mux_data.is_some() {
                            let existing_mux_data = existing_mux_data.unwrap();
                            let _ = client
                                .delete(format!(
                                    "https://api.mux.com/video/v1/assets/{}",
                                    existing_mux_data.clone().asset_id
                                ))
                                .basic_auth(mux_token_id.clone(), Some(mux_token_secret.clone()))
                                .send();

                            let existing_mux_data: mux_data::ActiveModel = existing_mux_data.into();
                            existing_mux_data.delete(db).await?;
                        }

                        let mux_body = MuxBody {
                            input: video_url.to_string(),
                            playback_policy: vec!["public".to_string()],
                            video_quality: "basic".to_string(),
                        };

                        let response = client
                            .post("https://api.mux.com/video/v1/assets")
                            .json(&mux_body)
                            .basic_auth(mux_token_id.clone(), Some(mux_token_secret.clone()))
                            .send()
                            .await
                            .unwrap();

                        match response.status() {
                            StatusCode::CREATED => {
                                let mux_response: MuxResponse = response.json().await.unwrap();
                                let new_mux_data = mux_data::ActiveModel {
                                    id: Set(Uuid::new_v4().to_string()),
                                    chapter_id: Set(chapter_id.clone()),
                                    asset_id: Set(mux_response.data.id),
                                    playback_id: Set(Some(
                                        mux_response.data.playback_ids[0].id.clone(),
                                    )),
                                    ..Default::default()
                                };

                                new_mux_data.insert(db).await?;
                            }
                            StatusCode::INTERNAL_SERVER_ERROR => {
                                return Err(DbErr::Custom("Internal server error".into()));
                            }
                            StatusCode::UNAUTHORIZED => {
                                return Err(DbErr::Custom(
                                    "Unauthorized for creating Mux Data".into(),
                                ));
                            }
                            _ => {
                                return Err(DbErr::Custom("Cannot create mux data".into()));
                            }
                        }
                    }
                }
                "title" => {
                    if let Some(title) = value.as_str() {
                        chapter.title = Set(title.to_string());
                    }
                }
                _ => continue,
            }
        }

        chapter.update(db).await?;
        Ok(())
    }
}
