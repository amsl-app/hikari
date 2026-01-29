use hikari_config::module::content::{ContentExam, QuestionBloomLevel};
use rand::{rng, seq::SliceRandom};

pub mod error;
pub mod evaluation;
pub mod question;

type ExamQuestion = (String, ContentExam);

fn max_five_random_exam_questions(
    mut exam_questions: Vec<ExamQuestion>,
    level: QuestionBloomLevel,
) -> Vec<ExamQuestion> {
    exam_questions.shuffle(&mut rng());
    let (fitting, rest): (Vec<ExamQuestion>, Vec<ExamQuestion>) =
        exam_questions.into_iter().partition(|(_, exam)| exam.level == level);

    // Add more questions from rest if less than 5 fitting questions
    if fitting.len() < 5 {
        let mut result = fitting;
        for item in rest {
            if result.len() >= 5 {
                break;
            }
            result.push(item);
        }
        return result;
    }

    fitting.into_iter().take(5).collect()
}
