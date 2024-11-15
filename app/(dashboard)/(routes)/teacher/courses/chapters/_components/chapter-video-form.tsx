"use client";

import * as z from "zod";
import MuxPlayer from "@mux/mux-player-react";

import { Button } from "@/components/ui/button";
import { Pencil, PlusCircle, Video } from "lucide-react";
import { useState } from "react";
import toast from "react-hot-toast";

import { Chapter, MuxData } from "@prisma/client";
import { FileUpload } from "@/components/file-upload";
import { invoke } from "@tauri-apps/api/core";

interface ChapterVideoFormProps {
    initialData: Chapter & { muxData?: MuxData | null };
    courseId: string;
    chapterId: string;
};

// eslint-disable-next-line @typescript-eslint/no-unused-vars
const formSchema = z.object({
    videoUrl: z.string().min(1),
})

export const ChapterVideoForm = ({
    initialData,
    courseId,
    chapterId
}: ChapterVideoFormProps) => {
    const userId = "user_2n3IHnfFLi6yuQ5GZrtiNlbuMM2";
    const [isEditting, setIsEditting] = useState(false);
    const [videoUrl, setVideoUrl] = useState(initialData?.videoUrl || undefined);

    const toggleEdit = () => setIsEditting((current) => !current);

    const onSubmit = async (values: z.infer<typeof formSchema>) => {
        console.log(courseId);
        invoke("update_chapter", {
            userId,
            courseId,
            chapterId,
            updates: values
        }).then(() => {
            toast.success("Chapter updated");
            toggleEdit();
            setVideoUrl(values.videoUrl);
        }).catch(err => toast.error(err));
    }

    return (
        <div className="mt-6 border bg-slate-100 rounded-md p-4">
            <div className="font-medium flex items-center justify-between">
                Chapter Video
                <Button onClick={toggleEdit} variant="ghost">
                    {isEditting && (
                        <>Cancel</>
                    )}
                    {!isEditting && !videoUrl && (
                        <>
                            <PlusCircle className="h-4 w-4 mr-2"/>
                            Add a video
                        </>
                    )}
                    {!isEditting && videoUrl && (
                        <>
                        <Pencil className="h-4 w-4 mr-2"/>
                        Edit video
                        </>
                    )}
                </Button>
            </div>
            {!isEditting && (
                !initialData.videoUrl ? (
                    <div className="flex items-center justify-center h-60 bg-slate-200 rounded-md">
                        <Video className="h-10 w-10 text-slate-500"/>
                    </div>
                ) : (
                    <div className="relative aspect-video mt-2">
                        <MuxPlayer
                            playbackId={initialData?.muxData?.playbackId || ""}
                        />
                    </div>
                )
            )}
            {isEditting && (
                <div>
                    <FileUpload
                        endpoint="chapterVideo"
                        onChange={(url) => {
                            console.log(url);
                            console.log("Lmao");
                            if (url) {
                                onSubmit({ videoUrl: url });
                            }
                        }}
                    />
                    <div className="text-xs text-muted-foreground mt-4">
                        Upload this chapter&apos;s video
                    </div>
                </div>
            )}
            {initialData.videoUrl && !isEditting && (
                <div className="text-xs text-muted-foreground mt-2">
                    Videos can take a few minutes to process. Refresh the page if video does not appear.
                </div>
            )}
        </div>
    )
}