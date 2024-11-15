import { db } from "@/lib/db";
import { isTeacher } from "@/lib/teacher";
import { NextResponse } from "next/server";

export async function POST(req:Request) {
    try {
        // const { userId } = auth();
        const userId = "user_2n3IHnfFLi6yuQ5GZrtiNlbuMM2";
        const { title } = await req.json();

        if (!userId || !isTeacher(userId)) {
            return new NextResponse("Unauthorized", { status: 401 });
        }

        const course = await db.course.create({
            data: {
                userId,
                title,
            }
        });

        return NextResponse.json(course);
    } catch (error) {
        console.log("[COURSES]", error);
        return new NextResponse("Internal Error", { status: 500 });
    }
}

export async function PATCH(req: Request) {
    try {
        // const { userId } = auth();
const userId = "user_2n3IHnfFLi6yuQ5GZrtiNlbuMM2";
        const { courseId, values } = await req.json();

        if (!userId) {
            return new NextResponse("Unauthorized", { status: 401 });
        }

        const course = await db.course.update({
            where: {
                id: courseId,
                userId
            },
            data: {
                ...values,
            }
        });

        return NextResponse.json(course);
    } catch (error) {
        console.log("[COURSE_ID]", error);
        return new NextResponse("Internal Error", { status: 500 });
    }
}
