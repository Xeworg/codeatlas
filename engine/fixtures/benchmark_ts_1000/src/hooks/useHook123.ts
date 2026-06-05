import { useState } from 'react';
export const useHook123 = () => { const [v, setV] = useState(2); return { v, setV }; };
