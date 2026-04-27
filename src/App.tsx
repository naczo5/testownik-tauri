import { useState, useEffect, useRef } from "react";
import "./App.css";
import { getBasesIndex, getBase, saveBase, getImageUrl, importOldBase, importNewBase, exportToAnki, BaseIndex, BaseData, Question } from "./api";
import { open } from "@tauri-apps/plugin-dialog";

type Screen = "select" | "quiz" | "results";
type ResultsMode = "session" | "completed";

interface QuestionStats {
  attempts: number;
  correct: number;
}

interface BaseProgress {
  answered: number;
  correct: number;
  total: number;
  questionStats: Record<string, QuestionStats>;
}

const PROGRESS_STORAGE_KEY = "testownik_progress";

const getQuestionId = (question: Question): string => {
  const questionId = question.id?.trim();
  if (questionId) return questionId;

  const questionText = question.question.trim();
  if (questionText) return questionText;

  return JSON.stringify(question.answers);
};

const buildProgressFromStats = (base: BaseData, existing?: BaseProgress, reset = false): BaseProgress => {
  if (reset || !existing || !existing.questionStats) {
    return { answered: 0, correct: 0, total: base.questionCount, questionStats: {} };
  }

  const questionIds = new Set(base.questions.map(getQuestionId));
  const normalizedQuestionStats: Record<string, QuestionStats> = {};

  for (const [questionId, stats] of Object.entries(existing.questionStats)) {
    if (!questionIds.has(questionId)) continue;

    const attempts = Math.max(0, Math.floor(stats?.attempts ?? 0));
    const rawCorrect = Math.max(0, Math.floor(stats?.correct ?? 0));
    const correct = attempts > 0 ? Math.min(rawCorrect, attempts) : 0;
    normalizedQuestionStats[questionId] = { attempts, correct };
  }

  let answered = 0;
  let correct = 0;

  for (const questionId of questionIds) {
    const stats = normalizedQuestionStats[questionId];
    if (!stats) continue;

    answered += stats.attempts;
    if (stats.correct > 0) correct += 1;
  }

  return {
    answered,
    correct,
    total: base.questionCount,
    questionStats: normalizedQuestionStats,
  };
};

const persistProgressData = (data: Record<string, BaseProgress>) => {
  localStorage.setItem(PROGRESS_STORAGE_KEY, JSON.stringify(data));
};

const cloneQuestion = (question: Question): Question => ({
  ...question,
  images: question.images ? [...question.images] : undefined,
  answers: question.answers.map(answer => ({ ...answer })),
  correct: [...question.correct],
});

const findQuestionIndex = (questions: Question[], targetQuestion: Question): number => {
  const targetId = targetQuestion.id?.trim();
  if (targetId) {
    const byId = questions.findIndex(question => question.id?.trim() === targetId);
    if (byId !== -1) return byId;
  }

  const targetAnswers = JSON.stringify(targetQuestion.answers);
  const byQuestionAndAnswers = questions.findIndex(
    question => question.question === targetQuestion.question && JSON.stringify(question.answers) === targetAnswers,
  );
  if (byQuestionAndAnswers !== -1) return byQuestionAndAnswers;

  const targetKey = getQuestionId(targetQuestion);
  const byKeyMatches = questions
    .map((question, index) => ({ index, key: getQuestionId(question) }))
    .filter(entry => entry.key === targetKey);

  if (byKeyMatches.length === 1) return byKeyMatches[0].index;

  return -1;
};

function App() {
  const [screen, setScreen] = useState<Screen>("select");
  const [bases, setBases] = useState<BaseIndex[]>([]);
  const [currentBase, setCurrentBase] = useState<BaseData | null>(null);
  const [loading, setLoading] = useState(false);
  
  // Quiz State
  const [shuffledQuestions, setShuffledQuestions] = useState<Question[]>([]);
  const [currentQuestionIndex, setCurrentQuestionIndex] = useState(0);
  const [selectedAnswers, setSelectedAnswers] = useState<Set<string>>(new Set());
  const [isEditing, setIsEditing] = useState(false);
  const [isAnswerChecked, setIsAnswerChecked] = useState(false);
  const [correctAnswersCount, setCorrectAnswersCount] = useState(0);
  const [resultsMode, setResultsMode] = useState<ResultsMode>("session");
  const [completedStats, setCompletedStats] = useState({ correct: 0, wrong: 0 });

  // Edit State
  const [editQuestionText, setEditQuestionText] = useState("");
  const [editAnswerTexts, setEditAnswerTexts] = useState<Record<string, string>>({});
  
  // Progress
  const [progressData, setProgressData] = useState<Record<string, BaseProgress>>({});
  const imageRequestIdRef = useRef(0);

  useEffect(() => {
    loadBases();
    loadProgress();
  }, []);

  const loadBases = async () => {
    try {
      const data = await getBasesIndex();
      setBases(data);
    } catch (err) {
      console.error(err);
    }
  };

  const loadProgress = () => {
    const data = localStorage.getItem(PROGRESS_STORAGE_KEY);
    if (data) {
      try {
        setProgressData(JSON.parse(data));
      } catch (e) {
        console.error("Failed to parse progress data", e);
      }
    }
  };

  const startQuiz = async (slug: string, forceRestart = false) => {
    try {
      const data = await getBase(slug);
      setCurrentBase(data);

      const prog = buildProgressFromStats(data, progressData[slug], forceRestart);
      setProgressData(prev => {
        const nextData = { ...prev, [slug]: prog };
        persistProgressData(nextData);
        return nextData;
      });

      const remainingQuestions = data.questions.filter(q => {
        const stats = prog.questionStats[getQuestionId(q)];
        return !stats || stats.correct === 0;
      });

      if (remainingQuestions.length === 0) {
        const completedCorrect = Math.min(data.questionCount, prog.correct);
        const completedWrong = Math.max(0, prog.answered - completedCorrect);

        setResultsMode("completed");
        setCompletedStats({ correct: completedCorrect, wrong: completedWrong });
        setShuffledQuestions([]);
        setCurrentQuestionIndex(0);
        setSelectedAnswers(new Set());
        setIsAnswerChecked(false);
        setIsEditing(false);
        setCorrectAnswersCount(0);
        setScreen("results");
        return;
      }

      // Shuffle questions
      const shuffled = [...remainingQuestions].sort(() => Math.random() - 0.5);
      setShuffledQuestions(shuffled);
      setCurrentQuestionIndex(0);
      setSelectedAnswers(new Set());
      setIsAnswerChecked(false);
      setIsEditing(false);
      setCorrectAnswersCount(0); // This is for the *current session*
      setResultsMode("session");
      setCompletedStats({ correct: 0, wrong: 0 });
      setScreen("quiz");
    } catch (err) {
      console.error(err);
      alert("Błąd podczas ładowania bazy.");
    }
  };

  const handleAnswerSelect = (key: string) => {
    if (!isEditing && isAnswerChecked) return;
    const newSelected = new Set(selectedAnswers);
    if (newSelected.has(key)) {
      newSelected.delete(key);
    } else {
      newSelected.add(key);
    }
    setSelectedAnswers(newSelected);
  };

  const checkAnswer = () => {
    if (selectedAnswers.size === 0 || isAnswerChecked) return;
    setIsAnswerChecked(true);
    
    const currentQ = shuffledQuestions[currentQuestionIndex];
    const isCorrect = 
      currentQ.correct.length === selectedAnswers.size && 
      currentQ.correct.every(c => selectedAnswers.has(c));
      
    if (isCorrect) {
      setCorrectAnswersCount(c => c + 1);
    }
    
    if (currentBase) {
      setProgressData(prev => {
         const slug = currentBase.slug;
         const prog = prev[slug] || { answered: 0, correct: 0, total: currentBase.questionCount, questionStats: {} };
         const safeQuestionStats = prog.questionStats || {};
         const qId = getQuestionId(currentQ);
         const qStats = safeQuestionStats[qId] || { attempts: 0, correct: 0 };
         const updatedQuestionStats = {
           ...safeQuestionStats,
           [qId]: {
             attempts: qStats.attempts + 1,
             correct: qStats.correct + (isCorrect ? 1 : 0)
           }
         };
         const answered = Object.values(updatedQuestionStats).reduce((sum, stats) => sum + stats.attempts, 0);
         const correct = Object.values(updatedQuestionStats).reduce((sum, stats) => sum + (stats.correct > 0 ? 1 : 0), 0);

          const newProg: BaseProgress = {
           ...prog,
           answered,
           correct: Math.min(correct, currentBase.questionCount),
           total: currentBase.questionCount,
           questionStats: updatedQuestionStats
          };
          
          const nextData = { ...prev, [slug]: newProg };
          persistProgressData(nextData);
          return nextData;
      });
    }
  };

  const nextQuestion = () => {
    if (currentQuestionIndex < shuffledQuestions.length - 1) {
      setCurrentQuestionIndex(i => i + 1);
      setSelectedAnswers(new Set());
      setIsAnswerChecked(false);
    } else {
      setResultsMode("session");
      setScreen("results");
    }
  };

  const toggleEdit = () => {
    if (isEditing) {
      setIsEditing(false);
    } else {
      setIsEditing(true);
      const currentQ = shuffledQuestions[currentQuestionIndex];
      setSelectedAnswers(new Set(currentQ.correct));
      setEditQuestionText(currentQ.question);
      const answerTexts: Record<string, string> = {};
      currentQ.answers.forEach(a => {
        if (!a.image) answerTexts[a.key] = a.text || "";
      });
      setEditAnswerTexts(answerTexts);
    }
  };

  const saveEditedQuestion = async () => {
    if (!currentBase) return;
    const currentQ = shuffledQuestions[currentQuestionIndex];
    
    const newCorrect = Array.from(selectedAnswers).sort();
    const newBase: BaseData = {
      ...currentBase,
      questions: currentBase.questions.map(cloneQuestion),
    };
    const qIndex = findQuestionIndex(newBase.questions, currentQ);
    if (qIndex === -1) {
      alert("Nie udało się jednoznacznie odnaleźć edytowanego pytania. Zapis został przerwany.");
      return;
    }

    newBase.questions[qIndex].correct = newCorrect;
    newBase.questions[qIndex].question = editQuestionText;
    newBase.questions[qIndex].answers = newBase.questions[qIndex].answers.map(a => {
      if (!a.image && editAnswerTexts[a.key] !== undefined) {
         return { ...a, text: editAnswerTexts[a.key] };
      }
      return a;
    });
    
    try {
      await saveBase(currentBase.slug, newBase);
      setCurrentBase(newBase);
      const updatedShuffled = [...shuffledQuestions];
      updatedShuffled[currentQuestionIndex] = newBase.questions[qIndex];
      setShuffledQuestions(updatedShuffled);
      setIsEditing(false);
      setIsAnswerChecked(false);
      setSelectedAnswers(new Set());
      alert("Zapisano zmiany!");
    } catch (err) {
      alert("Błąd zapisu: " + err);
    }
  };

  const handleImportOld = async () => {
    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: "Wybierz katalog ze starą bazą (.txt)",
      });
      if (selectedPath) {
        setLoading(true);
        await importOldBase(selectedPath as string);
        await loadBases();
        alert("Baza została zaimportowana!");
      }
    } catch (e) {
      alert(`Błąd: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleImportNew = async () => {
    try {
      const selectedPath = await open({
        directory: false,
        multiple: false,
        title: "Wybierz plik z nową bazą (.json)",
        filters: [{ name: 'JSON', extensions: ['json'] }]
      });
      if (selectedPath) {
        setLoading(true);
        await importNewBase(selectedPath as string);
        await loadBases();
        alert("Baza została zaimportowana!");
      }
    } catch (e) {
      alert(`Błąd: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleExportAnki = async (slug: string, e: React.MouseEvent) => {
    e.stopPropagation(); // don't trigger quiz start
    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: "Wybierz katalog do eksportu Anki",
      });
      if (selectedPath) {
        setLoading(true);
        await exportToAnki(slug, selectedPath as string);
        alert(`Eksport zakończony!\nPlik .txt zapisany w ${selectedPath}\nZdjecia skopiowane do folderu media/`);
      }
    } catch (err) {
      alert(`Błąd: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const currentQ = shuffledQuestions[currentQuestionIndex];
  
  const [imageUrls, setImageUrls] = useState<Record<string, string>>({});
  
  useEffect(() => {
    if (!currentBase || !currentQ) {
      setImageUrls({});
      return;
    }

    const requestId = imageRequestIdRef.current + 1;
    imageRequestIdRef.current = requestId;
    let isCancelled = false;
    setImageUrls({});

    const fetchImages = async () => {
      const urls: Record<string, string> = {};
      if (currentQ.images) {
        for (const img of currentQ.images) {
          urls[img] = await getImageUrl(currentBase.slug, img);
          if (isCancelled || requestId !== imageRequestIdRef.current) return;
        }
      }
      for (const a of currentQ.answers) {
        if (a.image) {
          urls[a.image] = await getImageUrl(currentBase.slug, a.image);
          if (isCancelled || requestId !== imageRequestIdRef.current) return;
        }
      }
      if (isCancelled || requestId !== imageRequestIdRef.current) return;
      setImageUrls(urls);
    };

    fetchImages().catch(error => {
      if (!isCancelled) {
        console.error("Failed to load images", error);
      }
    });

    return () => {
      isCancelled = true;
    };
  }, [currentBase, currentQ]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (screen !== "quiz" || isEditing) return;
      const currentQuestion = shuffledQuestions[currentQuestionIndex];
      if (!currentQuestion) return;
      const validAnswerKeys = new Set(currentQuestion.answers.map(answer => answer.key.toLowerCase()));

      const keyPressed = e.key.toLowerCase();
      if (!isAnswerChecked && validAnswerKeys.has(keyPressed)) {
        handleAnswerSelect(keyPressed);
      }

      const num = parseInt(e.key);
      if (!isNaN(num) && num >= 1 && num <= 9 && !isAnswerChecked) {
        if (num <= currentQuestion.answers.length) {
          handleAnswerSelect(currentQuestion.answers[num - 1].key);
        }
      }

      if (e.key === 'Enter' || e.key === ' ') {
        if (!isAnswerChecked && selectedAnswers.size > 0) {
          if (document.activeElement?.tagName !== 'BUTTON') {
            e.preventDefault();
            checkAnswer();
          }
        } else if (isAnswerChecked) {
          if (document.activeElement?.tagName !== 'BUTTON') {
            e.preventDefault();
            nextQuestion();
          }
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  });

  return (
    <div className="app">
      <header className="header">
        <div className="header-content">
          <h1 className="logo">📚 Testownik</h1>
        </div>
      </header>

      <main className="main">
        {screen === "select" && (
          <section className="screen screen-select">
            <h2 className="screen-title">Wybierz bazę pytań</h2>
            
            <div className="actions" style={{ marginBottom: '1.5rem', display: 'flex', gap: '0.5rem', justifyContent: 'center' }}>
                <button className="btn btn-secondary" onClick={handleImportOld} disabled={loading}>
                  {loading ? '⏳' : '📥 Importuj bazę (.txt)'}
                </button>
                <button className="btn btn-secondary" onClick={handleImportNew} disabled={loading}>
                  {loading ? '⏳' : '📥 Importuj bazę (.json)'}
                </button>
            </div>

            <div className="bases-grid">
              {bases.map(base => {
                const prog = progressData[base.slug] || { answered: 0, correct: 0, total: base.questionCount, questionStats: {} };
                const total = base.questionCount || 0;
                const statsCorrect = Object.values(prog.questionStats || {}).reduce((sum, stats) => sum + (stats.correct > 0 ? 1 : 0), 0);
                const correct = Math.min(total, Math.max(prog.correct || 0, statsCorrect));
                const percent = total > 0 ? Math.round((correct / total) * 100) : 0;
                const progressPercent = percent;
                const questionsLeft = Math.max(0, total - correct);
                const baseLabel = base.displayName?.trim() || base.name;
                
                return (
                  <div key={base.slug} className="base-card" onClick={() => startQuiz(base.slug)}>
                    <h3 className="base-title">{baseLabel}</h3>
                    <div className="base-meta">
                      <span>{base.questionCount} pytań • {questionsLeft} pozostało</span>
                    </div>
                    <div className="base-stats">
                      <div className="base-progress-bar">
                        <div className="base-progress-fill" style={{ width: `${progressPercent}%` }}></div>
                      </div>
                      <span className="base-progress-text">{correct}/{total} ({percent}%)</span>
                    </div>
                    <div style={{ marginTop: '1rem' }} onClick={e => e.stopPropagation()}>
                        <button 
                          className="btn btn-secondary" 
                          style={{ width: '100%', fontSize: '0.8rem', padding: '0.4rem' }}
                          onClick={(e) => handleExportAnki(base.slug, e)}
                          disabled={loading}
                        >
                            📤 Eksportuj do Anki
                        </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </section>
        )}

        {screen === "quiz" && currentQ && (
          <section className="screen screen-quiz">
            <div className="progress-container">
              <div className="progress-bar">
                <div 
                  className="progress-fill" 
                  style={{ width: `${((currentQuestionIndex) / Math.max(1, shuffledQuestions.length)) * 100}%` }}
                ></div>
              </div>
              <div className="progress-text">
                <span>{currentQuestionIndex + 1}</span> / <span>{shuffledQuestions.length}</span>
              </div>
            </div>

            <div className="question-card" style={isEditing ? { border: '2px dashed var(--primary)' } : {}}>
              <div className="question-number">Pytanie {currentQuestionIndex + 1}</div>
              <div className="question-text">
                {isEditing ? (
                  <textarea 
                    value={editQuestionText} 
                    onChange={e => setEditQuestionText(e.target.value)} 
                    style={{ width: '100%', minHeight: '80px', background: 'var(--bg-color)', color: 'inherit', border: '1px solid var(--border-color)', padding: '0.5rem', borderRadius: '4px', fontFamily: 'inherit', resize: 'vertical' }}
                  />
                ) : (
                  currentQ.question
                )}
              </div>
              <div className="question-images">
                {currentQ.images?.map((img, i) => (
                  imageUrls[img] ? <img key={i} src={imageUrls[img]} alt="Q" /> : null
                ))}
              </div>
            </div>

            <div className="answers-container">
              {currentQ.answers.map(ans => {
                const isSelected = selectedAnswers.has(ans.key);
                const isCorrect = currentQ.correct.includes(ans.key);
                
                let className = "answer-btn";
                if (isEditing) {
                   if (isSelected) className += " selected";
                } else if (isAnswerChecked) {
                   if (isCorrect) className += " correct";
                   else if (isSelected) className += " wrong";
                   else className += " disabled";
                } else {
                   if (isSelected) className += " selected";
                }

                return (
                  <div 
                    key={ans.key} 
                    className={className} 
                    onClick={() => handleAnswerSelect(ans.key)}
                  >
                    {isEditing && (
                      <input 
                        type="checkbox" 
                        className="edit-checkbox" 
                        checked={isSelected}
                        onChange={() => {}} 
                      />
                    )}
                    <span className="answer-key">{ans.key}</span>
                    <span className="answer-text">
                      {ans.image && imageUrls[ans.image] ? (
                        <img src={imageUrls[ans.image]} alt="A" className="answer-image" />
                      ) : isEditing ? (
                        <input
                           type="text"
                           value={editAnswerTexts[ans.key] !== undefined ? editAnswerTexts[ans.key] : ans.text}
                           onChange={e => setEditAnswerTexts(prev => ({...prev, [ans.key]: e.target.value}))}
                           style={{ flex: 1, background: 'var(--bg-color)', color: 'inherit', border: '1px solid var(--border-color)', padding: '0.3rem', borderRadius: '4px', fontFamily: 'inherit' }}
                           onClick={e => e.stopPropagation()}
                        />
                      ) : (
                        ans.text
                      )}
                    </span>
                  </div>
                );
              })}
            </div>

            <div className="actions">
              {isEditing ? (
                <>
                  <button className="btn btn-secondary" onClick={toggleEdit}>❌ Anuluj</button>
                  <button className="btn btn-primary" onClick={saveEditedQuestion}>💾 Zapisz</button>
                </>
              ) : (
                <>
                  <button className="btn btn-secondary" onClick={toggleEdit}>✏️ Edytuj</button>
                  <button className="btn btn-secondary" onClick={() => setScreen("select")}>← Wróć do listy</button>
                  {!isAnswerChecked && (
                    <button className="btn btn-primary" onClick={checkAnswer} disabled={selectedAnswers.size === 0}>
                      Sprawdź
                    </button>
                  )}
                  {isAnswerChecked && (
                    <button className="btn btn-primary" onClick={nextQuestion}>
                      Następne pytanie →
                    </button>
                  )}
                </>
              )}
            </div>
          </section>
        )}

        {screen === "results" && (
          <section className="screen screen-results">
             {(() => {
               const isCompleted = resultsMode === "completed";
               const resultCorrect = isCompleted ? completedStats.correct : correctAnswersCount;
               const resultWrong = isCompleted ? completedStats.wrong : Math.max(0, shuffledQuestions.length - correctAnswersCount);
               const resultPercent = Math.round((resultCorrect / Math.max(1, resultCorrect + resultWrong)) * 100);

               return (
              <div className="results-card">
                  <div className="results-icon">🎉</div>
                  <h2 className="results-title">{isCompleted ? "Baza ukończona!" : "Koniec tej serii!"}</h2>
                  <p className="results-subtitle">
                    {isCompleted
                      ? "Odpowiedziałeś poprawnie na wszystkie pytania. Możesz zresetować postęp i zacząć od nowa."
                      : "Zakończyłeś aktualnie rozwiązywane pytania z tej bazy."}
                  </p>
                  <div className="results-stats">
                      <div className="result-stat">
                          <span className="result-value">{resultCorrect}</span>
                          <span className="result-label">{isCompleted ? "Poprawne (łącznie)" : "Poprawne (teraz)"}</span>
                      </div>
                      <div className="result-stat">
                          <span className="result-value">{resultWrong}</span>
                          <span className="result-label">{isCompleted ? "Błędne próby (łącznie)" : "Błędne (teraz)"}</span>
                      </div>
                      <div className="result-stat">
                          <span className="result-value">{resultPercent}%</span>
                          <span className="result-label">Skuteczność</span>
                      </div>
                  </div>
                  <div className="results-actions">
                      <button className="btn btn-secondary" onClick={() => setScreen("select")}>← Wróć do listy</button>
                      <button className="btn btn-primary" onClick={() => currentBase && startQuiz(currentBase.slug, isCompleted)}>
                        {isCompleted ? "🔄 Zresetuj i zacznij od nowa" : "Kontynuuj resztę pytań →"}
                      </button>
                  </div>
              </div>
               );
             })()}
           </section>
        )}
      </main>
    </div>
  );
}

export default App;
