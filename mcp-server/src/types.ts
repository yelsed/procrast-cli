export interface Idea {
  id: number;
  uuid: string;
  userId: number;
  content: string;
  refinedContent: string | null;
  refinementSummary: string | null;
  summaryTitle: string | null;
  actionSteps: string[] | null;
  refinementStatus: string | null;
  refinementConversationId: string | null;
  audioPath: string | null;
  dueDate: string | null;
  priority: string | null;
  syncedAt: string | null;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
  completedAt: string | null;
}
