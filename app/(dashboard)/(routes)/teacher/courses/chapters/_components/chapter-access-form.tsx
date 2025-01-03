"use client";

import * as z from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";

import {
    Form,
    FormControl,
    FormDescription,
    FormField,
    FormItem
} from "@/components/ui/form"
import { Button } from "@/components/ui/button";
import { Pencil } from "lucide-react";
import { useState } from "react";
import toast from "react-hot-toast";
import { cn } from "@/lib/utils";
import { Chapter } from "@prisma/client";
import { Checkbox } from "@/components/ui/checkbox";
import { invoke } from "@tauri-apps/api/core";

interface ChapterAccessFormProps {
    initialData: Chapter;
    courseId: string;
    chapterId: string;
};

const formSchema = z.object({
    isFree: z.boolean().default(false)
})

export const ChapterAccessForm = ({
    initialData,
    courseId,
    chapterId
}: ChapterAccessFormProps) => {
    const userId = "user_2n3IHnfFLi6yuQ5GZrtiNlbuMM2";
    const [isEditting, setIsEditting] = useState(false);
    const [isCurrentlyFree, setIsFree] = useState(initialData.isFree);

    const toggleEdit = () => setIsEditting((current) => !current);

    const form = useForm<z.infer<typeof formSchema>>({
        resolver: zodResolver(formSchema),
        defaultValues: {
            isFree: Boolean(isCurrentlyFree)
        },
    })

    const { isSubmitting, isValid } = form.formState;

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
            setIsFree(values.isFree);
        }).catch(err => toast.error(err));
    }

    return (
        <div className="mt-6 border bg-slate-100 rounded-md p-4">
            <div className="font-medium flex items-center justify-between">
                Chapter Access
                <Button onClick={toggleEdit} variant="ghost">
                    {isEditting ? (
                        <>Cancel</>
                    ) : (
                        <>
                        <Pencil className="h-4 w-4 mr-2"/>
                        Edit access
                        </>
                    )}
                </Button>
            </div>
            {!isEditting && (
                <p className={cn(
                    "text-sm mt-2",
                    !isCurrentlyFree && "text-slate-500 italic"
                )}>
                    {isCurrentlyFree ? (
                        <>This chapter is free for preview.</>
                    ) : (
                        <>This chapter is not free.</>
                    )}
                </p>
            )}
            {isEditting && (
                <Form {...form}>
                    <form
                        onSubmit={form.handleSubmit(onSubmit)}
                        className="space-y-4 mt-4"
                    >
                        <FormField
                            control={form.control}
                            name="isFree"
                            render={({ field }) => (
                                <FormItem className="flex flex-row items-start space-x-3 space-y-0 rounded-md border p-4">
                                    <FormControl>
                                        <Checkbox
                                            checked={field.value}
                                            onCheckedChange={field.onChange}
                                        />
                                    </FormControl>
                                    <div className="space-y-1 leading-none">
                                        <FormDescription>
                                            Check this box if you want to make this chapter free for preview
                                        </FormDescription>
                                    </div>
                                </FormItem>
                            )}
                        />
                        <div className="flex items-center gap-x-2">
                            <Button
                                disabled={!isValid || isSubmitting}
                                type="submit"
                            >
                                Save
                            </Button>
                        </div>
                    </form>
                </Form>
            )}
        </div>
    )
}