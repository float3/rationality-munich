export const topics = [
 {id:'ea',name:'Effective altruism',description:'Socials, talks, and events about helping others effectively.'},
 {id:'rationality',name:'Rationality & clearer thinking',description:'LessWrong / ACX meetups, thinking games, and discussions.'},
 {id:'other',name:'Other community events',description:'Related philosophy, AI safety, and community invitations.'}
] as const;
export function validateSignup(email:string,selected:string[]){if(!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim()))return 'Enter a valid email address, such as you@example.com.';if(!selected.length)return 'Choose at least one topic to receive invitations about.';if(selected.some(id=>!topics.some(t=>t.id===id)))return 'Choose one of the available topics.';return ''}
