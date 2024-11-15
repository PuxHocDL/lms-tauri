import { db } from "@/lib/db";

import { NextResponse } from "next/server";

export async function PUT(
    req: Request
) {
    try {
        // const { userId } = auth();
        const userId = "user_2n3IHnfFLi6yuQ5GZrtiNlbuMM2";
        const { chapterId, isCompleted } = await req.json();

        if (!userId) {
            return new NextResponse("Unauthorized", { status: 401 });
        }

        const userProgress = await db.userProgress.upsert({
            where: {
                userId_chapterId: {
                    userId,
                    chapterId: chapterId
                }
            },
            update: {
                isCompleted
            },
            create: {
                userId,
                chapterId: chapterId,
                isCompleted
            }
        })

        return NextResponse.json(userProgress);
    } catch (error) {
        console.log("[CHAPTER_ID_PROGRESS]", error);
        return new NextResponse("Internal Error", { status: 500 });
    }
}