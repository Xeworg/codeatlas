import { useState } from 'react';
export const useHook002 = () => { const [v, setV] = useState(1); return { v, setV }; };
